package config

import (
	"fmt"
	"os"
	"strconv"
)

// Application holds application-level configuration.
type Application struct {
	AppName     string
	Environment string
	Port        int
}

// loadApplication loads application configuration from environment variables.
func loadApplication() (Application, error) {
	app := Application{}

	appName := os.Getenv("APP_NAME")
	if appName == "" {
		return app, fmt.Errorf("APP_NAME environment variable is required")
	}
	app.AppName = appName

	environment := os.Getenv("ENVIRONMENT")
	if environment == "" {
		return app, fmt.Errorf("ENVIRONMENT environment variable is required")
	}
	app.Environment = environment

	portStr := os.Getenv("PORT")
	if portStr == "" {
		return app, fmt.Errorf("PORT environment variable is required")
	}
	port, err := strconv.Atoi(portStr)
	if err != nil {
		return app, fmt.Errorf("PORT must be a valid integer: %w", err)
	}
	app.Port = port

	return app, nil
}

// Database holds database configuration.
type Database struct {
	Host     string
	Port     int
	Name     string
	User     string
	Password string
}

// loadDatabase loads database configuration from environment variables.
func loadDatabase() (Database, error) {
	db := Database{}

	host := os.Getenv("DB_HOST")
	if host == "" {
		return db, fmt.Errorf("DB_HOST environment variable is required")
	}
	db.Host = host

	portStr := os.Getenv("DB_PORT")
	if portStr == "" {
		return db, fmt.Errorf("DB_PORT environment variable is required")
	}
	port, err := strconv.Atoi(portStr)
	if err != nil {
		return db, fmt.Errorf("DB_PORT must be a valid integer: %w", err)
	}
	db.Port = port

	name := os.Getenv("DB_NAME")
	if name == "" {
		return db, fmt.Errorf("DB_NAME environment variable is required")
	}
	db.Name = name

	user := os.Getenv("DB_USER")
	if user == "" {
		return db, fmt.Errorf("DB_USER environment variable is required")
	}
	db.User = user

	password := os.Getenv("DB_PASSWORD")
	if password == "" {
		return db, fmt.Errorf("DB_PASSWORD environment variable is required")
	}
	db.Password = password

	return db, nil
}

// Caching holds caching configuration (e.g., Redis).
type Caching struct {
	Host     string
	Port     int
	Password string
	DB       int
}

// loadCaching loads caching configuration from environment variables.
func loadCaching() (Caching, error) {
	cache := Caching{}

	host := os.Getenv("CACHE_HOST")
	if host == "" {
		return cache, fmt.Errorf("CACHE_HOST environment variable is required")
	}
	cache.Host = host

	portStr := os.Getenv("CACHE_PORT")
	if portStr == "" {
		return cache, fmt.Errorf("CACHE_PORT environment variable is required")
	}
	port, err := strconv.Atoi(portStr)
	if err != nil {
		return cache, fmt.Errorf("CACHE_PORT must be a valid integer: %w", err)
	}
	cache.Port = port

	password := os.Getenv("CACHE_PASSWORD")
	cache.Password = password

	dbStr := os.Getenv("CACHE_DB")
	if dbStr == "" {
		return cache, fmt.Errorf("CACHE_DB environment variable is required")
	}
	db, err := strconv.Atoi(dbStr)
	if err != nil {
		return cache, fmt.Errorf("CACHE_DB must be a valid integer: %w", err)
	}
	cache.DB = db

	return cache, nil
}

// Messaging holds messaging configuration (e.g., message broker).
type Messaging struct {
	BrokerURL string
	Topic     string
	Queue     string
}

// loadMessaging loads messaging configuration from environment variables.
func loadMessaging() (Messaging, error) {
	msg := Messaging{}

	brokerURL := os.Getenv("MESSAGING_BROKER_URL")
	if brokerURL == "" {
		return msg, fmt.Errorf("MESSAGING_BROKER_URL environment variable is required")
	}
	msg.BrokerURL = brokerURL

	topic := os.Getenv("MESSAGING_TOPIC")
	if topic == "" {
		return msg, fmt.Errorf("MESSAGING_TOPIC environment variable is required")
	}
	msg.Topic = topic

	queue := os.Getenv("MESSAGING_QUEUE")
	if queue == "" {
		return msg, fmt.Errorf("MESSAGING_QUEUE environment variable is required")
	}
	msg.Queue = queue

	return msg, nil
}

// Config holds all application configuration.
type Config struct {
	Application Application
	Database    Database
	Caching     Caching
	Messaging   Messaging
}

// Load loads all configuration from environment variables.
// It returns a Config struct with all categories loaded or an error if any
// required configuration is missing or invalid.
func Load() (Config, error) {
	var cfg Config

	app, err := loadApplication()
	if err != nil {
		return cfg, err
	}
	cfg.Application = app

	db, err := loadDatabase()
	if err != nil {
		return cfg, err
	}
	cfg.Database = db

	cache, err := loadCaching()
	if err != nil {
		return cfg, err
	}
	cfg.Caching = cache

	msg, err := loadMessaging()
	if err != nil {
		return cfg, err
	}
	cfg.Messaging = msg

	return cfg, nil
}
