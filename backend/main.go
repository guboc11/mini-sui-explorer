package main

import (
	"context"
	"database/sql"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/joho/godotenv"
	_ "github.com/jackc/pgx/v5/stdlib"
)

type dbPinger interface {
	PingContext(ctx context.Context) error
}

func main() {
	_ = godotenv.Load()

	databaseURL := os.Getenv("DATABASE_URL")
	if databaseURL == "" {
		log.Fatal("DATABASE_URL is required")
	}

	db, err := sql.Open("pgx", databaseURL)
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	pingCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := db.PingContext(pingCtx); err != nil {
		log.Fatal(err)
	}

	router := setupRouter(db)

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	_ = router.Run(":" + port)
}

func setupRouter(db dbPinger) *gin.Engine {
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

	return router
}
