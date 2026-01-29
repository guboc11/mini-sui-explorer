package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
)

type fakeDB struct {
	err error
}

func (f fakeDB) PingContext(ctx context.Context) error {
	return f.err
}

func TestHealthOK(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := setupRouter(fakeDB{err: nil})

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	res := httptest.NewRecorder()
	router.ServeHTTP(res, req)

	if res.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", res.Code)
	}

	var body map[string]string
	if err := json.Unmarshal(res.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	if body["status"] != "ok" {
		t.Fatalf("expected status ok, got %q", body["status"])
	}
	if body["db"] != "ok" {
		t.Fatalf("expected db ok, got %q", body["db"])
	}
}

func TestHealthDBUnavailable(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := setupRouter(fakeDB{err: context.DeadlineExceeded})

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	res := httptest.NewRecorder()
	router.ServeHTTP(res, req)

	if res.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", res.Code)
	}

	var body map[string]string
	if err := json.Unmarshal(res.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	if body["db"] != "unavailable" {
		t.Fatalf("expected db unavailable, got %q", body["db"])
	}
}
