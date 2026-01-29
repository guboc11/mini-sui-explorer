package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/joho/godotenv"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

type dbHandle interface {
	PingContext(ctx context.Context) error
	ObjectTypeCounts(ctx context.Context, packageID string) ([]objectTypeCount, error)
}

type gormStore struct {
	db *gorm.DB
}

type objectTypeCount struct {
	ObjectType string `json:"object_type" gorm:"column:object_type"`
	Count      int64  `json:"count" gorm:"column:count"`
}

func (s gormStore) PingContext(ctx context.Context) error {
	sqlDB, err := s.db.DB()
	if err != nil {
		return err
	}
	return sqlDB.PingContext(ctx)
}

func (s gormStore) ObjectTypeCounts(ctx context.Context, packageID string) ([]objectTypeCount, error) {
	pattern := packageID + "::%"
	results := make([]objectTypeCount, 0)
	err := s.db.WithContext(ctx).
		Table("sui_objects").
		Select("object_type, COUNT(*) as count").
		Where("object_type IS NOT NULL AND object_type LIKE ?", pattern).
		Group("object_type").
		Order("object_type").
		Scan(&results).Error
	return results, err
}

func main() {
	_ = godotenv.Load()

	databaseURL := os.Getenv("DATABASE_URL")
	if databaseURL == "" {
		log.Fatal("DATABASE_URL is required")
	}

	db, err := gorm.Open(postgres.Open(databaseURL), &gorm.Config{})
	if err != nil {
		log.Fatal(err)
	}

	pingCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	store := gormStore{db: db}
	if err := store.PingContext(pingCtx); err != nil {
		log.Fatal(err)
	}

	router := setupRouter(store)

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	_ = router.Run(":" + port)
}

func setupRouter(db dbHandle) *gin.Engine {
	router := gin.Default()

	router.GET("/health", func(c *gin.Context) {
		ctx, cancel := context.WithTimeout(c.Request.Context(), 2*time.Second)
		defer cancel()

		dbStatus := "unavailable"
		if db != nil && db.PingContext(ctx) == nil {
			dbStatus = "ok"
		}

		c.JSON(http.StatusOK, gin.H{
			"status": "ok",
			"db":     dbStatus,
		})
	})

	router.GET("/packages/:packageId/objects", func(c *gin.Context) {
		if db == nil {
			c.JSON(http.StatusServiceUnavailable, gin.H{"error": "database unavailable"})
			return
		}

		packageID := c.Param("packageId")
		if packageID == "" {
			c.JSON(http.StatusBadRequest, gin.H{"error": "packageId is required"})
			return
		}

		ctx, cancel := context.WithTimeout(c.Request.Context(), 5*time.Second)
		defer cancel()

		results, err := db.ObjectTypeCounts(ctx, packageID)
		if err != nil {
			if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
				c.JSON(http.StatusRequestTimeout, gin.H{"error": "query timeout"})
				return
			}
			c.JSON(http.StatusInternalServerError, gin.H{"error": "query failed"})
			return
		}

		c.JSON(http.StatusOK, gin.H{
			"package_id": packageID,
			"types":      results,
		})
	})

	return router
}
