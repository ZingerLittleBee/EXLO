# Exlo Simple Deployment Makefile
# Usage: make <target>

COMPOSE_FILE := docker-compose.simple.yml

.PHONY: help db-init up up-build down logs ps clean

help: ## Show this help
	@echo "Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Typical workflow:"
	@echo "  1. make db-init   # First time: start database and run migrations"
	@echo "  2. make up-build  # Start web and tunnl services with rebuild"
	@echo "  3. make up        # Restart services without rebuild"
	@echo ""
	@echo "Tips:"
	@echo "  make up-build web      # Build and start only web service"
	@echo "  make up-build tunnl    # Build and start only tunnl service"
	@echo "  make up-build web tunnl # Build and start both (same as no args)"

db-init: ## Start database and run migrations (first time setup)
	@echo "Starting PostgreSQL and running migrations..."
	docker compose -f $(COMPOSE_FILE) --profile db up -d
	@echo "Database initialized successfully!"

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
