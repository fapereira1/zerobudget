package config

import (
	"os"
	"testing"
)

func TestLoadApplication(t *testing.T) {
	tests := []struct {
		name        string
		envVars     map[string]string
		wantErr     bool
		errContains string
		validate    func(t *testing.T, app Application)
	}{
		{
			name: "valid configuration",
			envVars: map[string]string{
				"APP_NAME":    "testapp",
				"ENVIRONMENT": "development",
				"PORT":        "8080",
			},
			wantErr: false,
			validate: func(t *testing.T, app Application) {
				if app.AppName != "testapp" {
					t.Errorf("AppName = %v, want testapp", app.AppName)
				}
				if app.Environment != "development" {
					t.Errorf("Environment = %v, want development", app.Environment)
				}
				if app.Port != 8080 {
					t.Errorf("Port = %v, want 8080", app.Port)
				}
			},
		},
		{
			name: "missing APP_NAME",
			envVars: map[string]string{
				"ENVIRONMENT": "development",
				"PORT":        "8080",
			},
			wantErr:     true,
			errContains: "APP_NAME",
		},
		{
			name: "missing ENVIRONMENT",
			envVars: map[string]string{
				"APP_NAME": "testapp",
				"PORT":     "8080",
			},
			wantErr:     true,
			errContains: "ENVIRONMENT",
		},
		{
			name: "missing PORT",
			envVars: map[string]string{
				"APP_NAME":    "testapp",
				"ENVIRONMENT": "development",
			},
			wantErr:     true,
			errContains: "PORT",
		},
		{
			name: "invalid PORT format",
			envVars: map[string]string{
				"APP_NAME":    "testapp",
				"ENVIRONMENT": "development",
				"PORT":        "not-a-number",
			},
			wantErr:     true,
			errContains: "PORT must be a valid integer",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Save original env vars
			originalEnv := make(map[string]string)
			for key := range tt.envVars {
				originalEnv[key] = os.Getenv(key)
			}

			// Clean up env vars
			os.Clearenv()

			// Set test env vars
			for key, value := range tt.envVars {
				os.Setenv(key, value)
			}

			// Run test
			app, err := loadApplication()

			// Restore original env vars
			os.Clearenv()
			for key, value := range originalEnv {
				if value != "" {
					os.Setenv(key, value)
				}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("loadApplication() error = nil, want error containing %q", tt.errContains)
				} else if tt.errContains != "" && !contains(err.Error(), tt.errContains) {
					t.Errorf("loadApplication() error = %v, want error containing %q", err, tt.errContains)
				}
				return
			}

			if err != nil {
				t.Errorf("loadApplication() error = %v, want nil", err)
				return
			}

			if tt.validate != nil {
				tt.validate(t, app)
			}
		})
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > len(substr) && (s[:len(substr)] == substr ||
			s[len(s)-len(substr):] == substr ||
			containsMiddle(s, substr))))
}

func containsMiddle(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

func TestLoadDatabase(t *testing.T) {
	tests := []struct {
		name        string
		envVars     map[string]string
		wantErr     bool
		errContains string
		validate    func(t *testing.T, db Database)
	}{
		{
			name: "valid configuration",
			envVars: map[string]string{
				"DB_HOST":     "localhost",
				"DB_PORT":     "5432",
				"DB_NAME":     "zerobudget",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret",
			},
			wantErr: false,
			validate: func(t *testing.T, db Database) {
				if db.Host != "localhost" {
					t.Errorf("Host = %v, want localhost", db.Host)
				}
				if db.Port != 5432 {
					t.Errorf("Port = %v, want 5432", db.Port)
				}
				if db.Name != "zerobudget" {
					t.Errorf("Name = %v, want zerobudget", db.Name)
				}
				if db.User != "postgres" {
					t.Errorf("User = %v, want postgres", db.User)
				}
				if db.Password != "secret" {
					t.Errorf("Password = %v, want secret", db.Password)
				}
			},
		},
		{
			name: "missing DB_HOST",
			envVars: map[string]string{
				"DB_PORT":     "5432",
				"DB_NAME":     "zerobudget",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret",
			},
			wantErr:     true,
			errContains: "DB_HOST",
		},
		{
			name: "missing DB_PORT",
			envVars: map[string]string{
				"DB_HOST":     "localhost",
				"DB_NAME":     "zerobudget",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret",
			},
			wantErr:     true,
			errContains: "DB_PORT",
		},
		{
			name: "missing DB_NAME",
			envVars: map[string]string{
				"DB_HOST":     "localhost",
				"DB_PORT":     "5432",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret",
			},
			wantErr:     true,
			errContains: "DB_NAME",
		},
		{
			name: "missing DB_USER",
			envVars: map[string]string{
				"DB_HOST":     "localhost",
				"DB_PORT":     "5432",
				"DB_NAME":     "zerobudget",
				"DB_PASSWORD": "secret",
			},
			wantErr:     true,
			errContains: "DB_USER",
		},
		{
			name: "missing DB_PASSWORD",
			envVars: map[string]string{
				"DB_HOST": "localhost",
				"DB_PORT": "5432",
				"DB_NAME": "zerobudget",
				"DB_USER": "postgres",
			},
			wantErr:     true,
			errContains: "DB_PASSWORD",
		},
		{
			name: "invalid DB_PORT format",
			envVars: map[string]string{
				"DB_HOST":     "localhost",
				"DB_PORT":     "not-a-number",
				"DB_NAME":     "zerobudget",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret",
			},
			wantErr:     true,
			errContains: "DB_PORT must be a valid integer",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Save original env vars
			originalEnv := make(map[string]string)
			for key := range tt.envVars {
				originalEnv[key] = os.Getenv(key)
			}

			// Clean up env vars
			os.Clearenv()

			// Set test env vars
			for key, value := range tt.envVars {
				os.Setenv(key, value)
			}

			// Run test
			db, err := loadDatabase()

			// Restore original env vars
			os.Clearenv()
			for key, value := range originalEnv {
				if value != "" {
					os.Setenv(key, value)
				}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("loadDatabase() error = nil, want error containing %q", tt.errContains)
				} else if tt.errContains != "" && !contains(err.Error(), tt.errContains) {
					t.Errorf("loadDatabase() error = %v, want error containing %q", err, tt.errContains)
				}
				return
			}

			if err != nil {
				t.Errorf("loadDatabase() error = %v, want nil", err)
				return
			}

			if tt.validate != nil {
				tt.validate(t, db)
			}
		})
	}
}

func TestLoadCaching(t *testing.T) {
	tests := []struct {
		name        string
		envVars     map[string]string
		wantErr     bool
		errContains string
		validate    func(t *testing.T, cache Caching)
	}{
		{
			name: "valid configuration with password",
			envVars: map[string]string{
				"CACHE_HOST":     "localhost",
				"CACHE_PORT":     "6379",
				"CACHE_PASSWORD": "redis-secret",
				"CACHE_DB":       "0",
			},
			wantErr: false,
			validate: func(t *testing.T, cache Caching) {
				if cache.Host != "localhost" {
					t.Errorf("Host = %v, want localhost", cache.Host)
				}
				if cache.Port != 6379 {
					t.Errorf("Port = %v, want 6379", cache.Port)
				}
				if cache.Password != "redis-secret" {
					t.Errorf("Password = %v, want redis-secret", cache.Password)
				}
				if cache.DB != 0 {
					t.Errorf("DB = %v, want 0", cache.DB)
				}
			},
		},
		{
			name: "valid configuration without password",
			envVars: map[string]string{
				"CACHE_HOST": "localhost",
				"CACHE_PORT": "6379",
				"CACHE_DB":   "1",
			},
			wantErr: false,
			validate: func(t *testing.T, cache Caching) {
				if cache.Host != "localhost" {
					t.Errorf("Host = %v, want localhost", cache.Host)
				}
				if cache.Port != 6379 {
					t.Errorf("Port = %v, want 6379", cache.Port)
				}
				if cache.Password != "" {
					t.Errorf("Password = %v, want empty string", cache.Password)
				}
				if cache.DB != 1 {
					t.Errorf("DB = %v, want 1", cache.DB)
				}
			},
		},
		{
			name: "missing CACHE_HOST",
			envVars: map[string]string{
				"CACHE_PORT": "6379",
				"CACHE_DB":   "0",
			},
			wantErr:     true,
			errContains: "CACHE_HOST",
		},
		{
			name: "missing CACHE_PORT",
			envVars: map[string]string{
				"CACHE_HOST": "localhost",
				"CACHE_DB":   "0",
			},
			wantErr:     true,
			errContains: "CACHE_PORT",
		},
		{
			name: "missing CACHE_DB",
			envVars: map[string]string{
				"CACHE_HOST": "localhost",
				"CACHE_PORT": "6379",
			},
			wantErr:     true,
			errContains: "CACHE_DB",
		},
		{
			name: "invalid CACHE_PORT format",
			envVars: map[string]string{
				"CACHE_HOST": "localhost",
				"CACHE_PORT": "not-a-number",
				"CACHE_DB":   "0",
			},
			wantErr:     true,
			errContains: "CACHE_PORT must be a valid integer",
		},
		{
			name: "invalid CACHE_DB format",
			envVars: map[string]string{
				"CACHE_HOST": "localhost",
				"CACHE_PORT": "6379",
				"CACHE_DB":   "not-a-number",
			},
			wantErr:     true,
			errContains: "CACHE_DB must be a valid integer",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Save original env vars
			originalEnv := make(map[string]string)
			for key := range tt.envVars {
				originalEnv[key] = os.Getenv(key)
			}

			// Clean up env vars
			os.Clearenv()

			// Set test env vars
			for key, value := range tt.envVars {
				os.Setenv(key, value)
			}

			// Run test
			cache, err := loadCaching()

			// Restore original env vars
			os.Clearenv()
			for key, value := range originalEnv {
				if value != "" {
					os.Setenv(key, value)
				}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("loadCaching() error = nil, want error containing %q", tt.errContains)
				} else if tt.errContains != "" && !contains(err.Error(), tt.errContains) {
					t.Errorf("loadCaching() error = %v, want error containing %q", err, tt.errContains)
				}
				return
			}

			if err != nil {
				t.Errorf("loadCaching() error = %v, want nil", err)
				return
			}

			if tt.validate != nil {
				tt.validate(t, cache)
			}
		})
	}
}

func TestLoadMessaging(t *testing.T) {
	tests := []struct {
		name        string
		envVars     map[string]string
		wantErr     bool
		errContains string
		validate    func(t *testing.T, msg Messaging)
	}{
		{
			name: "valid configuration",
			envVars: map[string]string{
				"MESSAGING_BROKER_URL": "amqp://localhost:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr: false,
			validate: func(t *testing.T, msg Messaging) {
				if msg.BrokerURL != "amqp://localhost:5672" {
					t.Errorf("BrokerURL = %v, want amqp://localhost:5672", msg.BrokerURL)
				}
				if msg.Topic != "events" {
					t.Errorf("Topic = %v, want events", msg.Topic)
				}
				if msg.Queue != "notifications" {
					t.Errorf("Queue = %v, want notifications", msg.Queue)
				}
			},
		},
		{
			name: "missing MESSAGING_BROKER_URL",
			envVars: map[string]string{
				"MESSAGING_TOPIC": "events",
				"MESSAGING_QUEUE": "notifications",
			},
			wantErr:     true,
			errContains: "MESSAGING_BROKER_URL",
		},
		{
			name: "missing MESSAGING_TOPIC",
			envVars: map[string]string{
				"MESSAGING_BROKER_URL": "amqp://localhost:5672",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr:     true,
			errContains: "MESSAGING_TOPIC",
		},
		{
			name: "missing MESSAGING_QUEUE",
			envVars: map[string]string{
				"MESSAGING_BROKER_URL": "amqp://localhost:5672",
				"MESSAGING_TOPIC":      "events",
			},
			wantErr:     true,
			errContains: "MESSAGING_QUEUE",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Save original env vars
			originalEnv := make(map[string]string)
			for key := range tt.envVars {
				originalEnv[key] = os.Getenv(key)
			}

			// Clean up env vars
			os.Clearenv()

			// Set test env vars
			for key, value := range tt.envVars {
				os.Setenv(key, value)
			}

			// Run test
			msg, err := loadMessaging()

			// Restore original env vars
			os.Clearenv()
			for key, value := range originalEnv {
				if value != "" {
					os.Setenv(key, value)
				}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("loadMessaging() error = nil, want error containing %q", tt.errContains)
				} else if tt.errContains != "" && !contains(err.Error(), tt.errContains) {
					t.Errorf("loadMessaging() error = %v, want error containing %q", err, tt.errContains)
				}
				return
			}

			if err != nil {
				t.Errorf("loadMessaging() error = %v, want nil", err)
				return
			}

			if tt.validate != nil {
				tt.validate(t, msg)
			}
		})
	}
}

func TestLoad(t *testing.T) {
	tests := []struct {
		name        string
		envVars     map[string]string
		wantErr     bool
		errContains string
		validate    func(t *testing.T, cfg Config)
	}{
		{
			name: "valid complete configuration",
			envVars: map[string]string{
				// Application
				"APP_NAME":    "zerobudget",
				"ENVIRONMENT": "production",
				"PORT":        "8080",
				// Database
				"DB_HOST":     "db.example.com",
				"DB_PORT":     "5432",
				"DB_NAME":     "zerobudget",
				"DB_USER":     "postgres",
				"DB_PASSWORD": "secret123",
				// Caching
				"CACHE_HOST":     "cache.example.com",
				"CACHE_PORT":     "6379",
				"CACHE_PASSWORD": "redis-secret",
				"CACHE_DB":       "0",
				// Messaging
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr: false,
			validate: func(t *testing.T, cfg Config) {
				// Validate Application
				if cfg.Application.AppName != "zerobudget" {
					t.Errorf("Application.AppName = %v, want zerobudget", cfg.Application.AppName)
				}
				if cfg.Application.Environment != "production" {
					t.Errorf("Application.Environment = %v, want production", cfg.Application.Environment)
				}
				if cfg.Application.Port != 8080 {
					t.Errorf("Application.Port = %v, want 8080", cfg.Application.Port)
				}
				// Validate Database
				if cfg.Database.Host != "db.example.com" {
					t.Errorf("Database.Host = %v, want db.example.com", cfg.Database.Host)
				}
				if cfg.Database.Port != 5432 {
					t.Errorf("Database.Port = %v, want 5432", cfg.Database.Port)
				}
				if cfg.Database.Name != "zerobudget" {
					t.Errorf("Database.Name = %v, want zerobudget", cfg.Database.Name)
				}
				if cfg.Database.User != "postgres" {
					t.Errorf("Database.User = %v, want postgres", cfg.Database.User)
				}
				if cfg.Database.Password != "secret123" {
					t.Errorf("Database.Password = %v, want secret123", cfg.Database.Password)
				}
				// Validate Caching
				if cfg.Caching.Host != "cache.example.com" {
					t.Errorf("Caching.Host = %v, want cache.example.com", cfg.Caching.Host)
				}
				if cfg.Caching.Port != 6379 {
					t.Errorf("Caching.Port = %v, want 6379", cfg.Caching.Port)
				}
				if cfg.Caching.Password != "redis-secret" {
					t.Errorf("Caching.Password = %v, want redis-secret", cfg.Caching.Password)
				}
				if cfg.Caching.DB != 0 {
					t.Errorf("Caching.DB = %v, want 0", cfg.Caching.DB)
				}
				// Validate Messaging
				if cfg.Messaging.BrokerURL != "amqp://broker.example.com:5672" {
					t.Errorf("Messaging.BrokerURL = %v, want amqp://broker.example.com:5672", cfg.Messaging.BrokerURL)
				}
				if cfg.Messaging.Topic != "events" {
					t.Errorf("Messaging.Topic = %v, want events", cfg.Messaging.Topic)
				}
				if cfg.Messaging.Queue != "notifications" {
					t.Errorf("Messaging.Queue = %v, want notifications", cfg.Messaging.Queue)
				}
			},
		},
		{
			name: "missing Application variable",
			envVars: map[string]string{
				"ENVIRONMENT":          "production",
				"PORT":                 "8080",
				"DB_HOST":              "db.example.com",
				"DB_PORT":              "5432",
				"DB_NAME":              "zerobudget",
				"DB_USER":              "postgres",
				"DB_PASSWORD":          "secret123",
				"CACHE_HOST":           "cache.example.com",
				"CACHE_PORT":           "6379",
				"CACHE_DB":             "0",
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr:     true,
			errContains: "APP_NAME",
		},
		{
			name: "missing Database variable",
			envVars: map[string]string{
				"APP_NAME":             "zerobudget",
				"ENVIRONMENT":          "production",
				"PORT":                 "8080",
				"DB_HOST":              "db.example.com",
				"DB_PORT":              "5432",
				"DB_NAME":              "zerobudget",
				"DB_USER":              "postgres",
				"CACHE_HOST":           "cache.example.com",
				"CACHE_PORT":           "6379",
				"CACHE_DB":             "0",
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr:     true,
			errContains: "DB_PASSWORD",
		},
		{
			name: "missing Caching variable",
			envVars: map[string]string{
				"APP_NAME":             "zerobudget",
				"ENVIRONMENT":          "production",
				"PORT":                 "8080",
				"DB_HOST":              "db.example.com",
				"DB_PORT":              "5432",
				"DB_NAME":              "zerobudget",
				"DB_USER":              "postgres",
				"DB_PASSWORD":          "secret123",
				"CACHE_HOST":           "cache.example.com",
				"CACHE_PORT":           "6379",
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr:     true,
			errContains: "CACHE_DB",
		},
		{
			name: "missing Messaging variable",
			envVars: map[string]string{
				"APP_NAME":             "zerobudget",
				"ENVIRONMENT":          "production",
				"PORT":                 "8080",
				"DB_HOST":              "db.example.com",
				"DB_PORT":              "5432",
				"DB_NAME":              "zerobudget",
				"DB_USER":              "postgres",
				"DB_PASSWORD":          "secret123",
				"CACHE_HOST":           "cache.example.com",
				"CACHE_PORT":           "6379",
				"CACHE_DB":             "0",
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
			},
			wantErr:     true,
			errContains: "MESSAGING_QUEUE",
		},
		{
			name: "invalid Application variable format",
			envVars: map[string]string{
				"APP_NAME":             "zerobudget",
				"ENVIRONMENT":          "production",
				"PORT":                 "not-a-number",
				"DB_HOST":              "db.example.com",
				"DB_PORT":              "5432",
				"DB_NAME":              "zerobudget",
				"DB_USER":              "postgres",
				"DB_PASSWORD":          "secret123",
				"CACHE_HOST":           "cache.example.com",
				"CACHE_PORT":           "6379",
				"CACHE_DB":             "0",
				"MESSAGING_BROKER_URL": "amqp://broker.example.com:5672",
				"MESSAGING_TOPIC":      "events",
				"MESSAGING_QUEUE":      "notifications",
			},
			wantErr:     true,
			errContains: "PORT must be a valid integer",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Save original env vars
			originalEnv := make(map[string]string)
			for key := range tt.envVars {
				originalEnv[key] = os.Getenv(key)
			}

			// Clean up env vars
			os.Clearenv()

			// Set test env vars
			for key, value := range tt.envVars {
				os.Setenv(key, value)
			}

			// Run test
			cfg, err := Load()

			// Restore original env vars
			os.Clearenv()
			for key, value := range originalEnv {
				if value != "" {
					os.Setenv(key, value)
				}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("Load() error = nil, want error containing %q", tt.errContains)
				} else if tt.errContains != "" && !contains(err.Error(), tt.errContains) {
					t.Errorf("Load() error = %v, want error containing %q", err, tt.errContains)
				}
				return
			}

			if err != nil {
				t.Errorf("Load() error = %v, want nil", err)
				return
			}

			if tt.validate != nil {
				tt.validate(t, cfg)
			}
		})
	}
}
