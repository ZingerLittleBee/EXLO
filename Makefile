# Exlo Simple Deployment Makefile
# Usage: make <target>

COMPOSE_FILE := docker-compose.simple.yml
COMPOSE_LOCAL := docker-compose.local.yml
COMPOSE_PROD := docker-compose.yml

.PHONY: help menu init-db init-dev-db dev-web dev-docs dev-landing dev-tunnl dev-all \
        build build-web build-tunnl build-images \
        deploy-simple deploy-local deploy-prod \
        up up-build down logs logs-web logs-tunnl ps \
        db-backup db-restore setup-env validate-env health status \
        clean clean-images clean-volumes

.DEFAULT_GOAL := menu

help: ## Show this help
	@echo "Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Typical workflow:"
	@echo "  1. make init-db   # First time: start database and run migrations"
	@echo "  2. make up-build  # Start web and tunnl services with rebuild"
	@echo "  3. make up        # Restart services without rebuild"
	@echo ""
	@echo "Tips:"
	@echo "  make menu              # Interactive menu for all commands"
	@echo "  make up-build web      # Build and start only web service"
	@echo "  make up-build tunnl    # Build and start only tunnl service"

menu: ## Launch interactive command menu (requires gum)
	@./scripts/menu.sh

init-db: ## Start database and run migrations (first time setup)
	@echo "Starting PostgreSQL and running migrations..."
	docker compose -f $(COMPOSE_FILE) --profile db up -d
	@echo "Database initialized successfully!"

init-dev-db: ## Start dev database and push schema
	@echo "Starting dev PostgreSQL..."
	docker compose -f packages/db/docker-compose.yml up -d
	@echo "Pushing database schema..."
	bun db:push
	@echo "Dev database initialized!"

dev-web: ## Run web in dev mode
	bun dev:web

dev-docs: ## Run docs in dev mode
	bun dev:docs

dev-tunnl: ## Run tunnl in dev mode
	cd apps/tunnl && RUST_LOG=info cargo run

dev-landing: ## Run landing page in dev mode
	bun dev:landing

dev-all: ## Run all dev services in parallel
	@echo "Starting all development services..."
	@trap 'kill 0' EXIT; \
	bun dev:web & \
	bun dev:landing & \
	bun dev:docs & \
	(cd apps/tunnl && RUST_LOG=info cargo run) & \
	wait

# ==================== Build ====================

build: ## Build all applications
	@echo "Building all applications..."
	bun build
	@echo "Build complete!"

build-web: ## Build web application only
	@echo "Building web application..."
	cd apps/web && bun build
	@echo "Web build complete!"

build-tunnl: ## Build tunnl (Rust) only
	@echo "Building tunnl..."
	cd apps/tunnl && cargo build --release
	@echo "Tunnl build complete!"

build-images: ## Build all Docker images
	@echo "Building Docker images..."
	docker compose -f $(COMPOSE_FILE) build
	@echo "Docker images built!"

# ==================== Deploy ====================

deploy-simple: ## Deploy using simple config (HTTP, no domain)
	@echo "Deploying with simple configuration..."
	docker compose -f $(COMPOSE_FILE) --profile db up -d
	@echo "Simple deployment complete!"

deploy-local: ## Deploy using local config (Traefik HTTP)
	@echo "Deploying with local configuration..."
	docker compose -f $(COMPOSE_LOCAL) --profile db up -d
	@echo "Local deployment complete! Traefik dashboard: http://localhost:8081"

deploy-prod: ## Deploy using production config (HTTPS + Let's Encrypt)
	@echo "Deploying with production configuration..."
	docker compose -f $(COMPOSE_PROD) --profile db up -d
	@echo "Production deployment complete!"

# ==================== Services ====================

up: ## Start web and tunnl services (without rebuild)
	@echo "Starting web and tunnl services..."
	docker compose -f $(COMPOSE_FILE) up -d web tunnl
	@echo "Services started!"

up-build: ## Start services with rebuild (usage: make up-build [web] [tunnl])
	$(eval SERVICES := $(if $(filter web tunnl,$(MAKECMDGOALS)),$(filter web tunnl,$(MAKECMDGOALS)),web tunnl))
	@echo "Building and starting services: $(SERVICES)..."
	docker compose -f $(COMPOSE_FILE) up -d --build $(SERVICES)
	@echo "Services started!"

web tunnl:
	@:

down: ## Stop all services
	@echo "Stopping all services..."
	docker compose -f $(COMPOSE_FILE) --profile db down
	@echo "All services stopped."

logs: ## Show logs for all services
	docker compose -f $(COMPOSE_FILE) logs -f

logs-web: ## Show logs for web service
	docker compose -f $(COMPOSE_FILE) logs -f web

logs-tunnl: ## Show logs for tunnl service
	docker compose -f $(COMPOSE_FILE) logs -f tunnl

ps: ## Show running containers
	docker compose -f $(COMPOSE_FILE) ps

clean: ## Stop and remove all containers, volumes, and images
	@echo "WARNING: This will remove all data including the database!"
	@read -p "Are you sure? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	docker compose -f $(COMPOSE_FILE) --profile db down -v --rmi local
	@echo "Cleanup complete."

clean-images: ## Remove Docker images only
	@echo "Removing Docker images..."
	docker compose -f $(COMPOSE_FILE) down --rmi local
	@echo "Images removed!"

clean-volumes: ## Remove Docker volumes only
	@echo "WARNING: This will remove all data!"
	@read -p "Are you sure? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	docker compose -f $(COMPOSE_FILE) down -v
	@echo "Volumes removed!"

# ==================== Database ====================

db-backup: ## Backup PostgreSQL database
	@echo "Backing up database..."
	@mkdir -p backups
	docker compose -f $(COMPOSE_FILE) exec -T postgres pg_dump -U postgres exlo > backups/exlo_$$(date +%Y%m%d_%H%M%S).sql
	@echo "Backup saved to backups/"

db-restore: ## Restore PostgreSQL database (usage: make db-restore FILE=backups/xxx.sql)
	@if [ -z "$(FILE)" ]; then echo "Usage: make db-restore FILE=backups/xxx.sql"; exit 1; fi
	@echo "Restoring database from $(FILE)..."
	docker compose -f $(COMPOSE_FILE) exec -T postgres psql -U postgres exlo < $(FILE)
	@echo "Database restored!"

# ==================== Environment ====================

setup-env: ## Create .env file from template
	@if [ -f .env ]; then \
		echo ".env file already exists. Skipping..."; \
	else \
		cp .env.example .env; \
		echo ".env file created from .env.example"; \
		echo "Please edit .env with your settings."; \
	fi

validate-env: ## Validate required environment variables
	@echo "Validating environment variables..."
	@missing=""; \
	for var in POSTGRES_DB POSTGRES_USER POSTGRES_PASSWORD BETTER_AUTH_SECRET; do \
		if ! grep -q "^$$var=" .env 2>/dev/null; then \
			missing="$$missing $$var"; \
		fi; \
	done; \
	if [ -n "$$missing" ]; then \
		echo "Missing variables:$$missing"; \
		exit 1; \
	fi
	@echo "All required variables are set!"

# ==================== Health & Status ====================

health: ## Check health of all services
	@echo "Checking service health..."
	@docker compose -f $(COMPOSE_FILE) ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"

status: ## Show detailed status information
	@echo "=== Docker Containers ==="
	@docker compose -f $(COMPOSE_FILE) ps
	@echo ""
	@echo "=== Resource Usage ==="
	@docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" $$(docker compose -f $(COMPOSE_FILE) ps -q) 2>/dev/null || echo "No running containers"
