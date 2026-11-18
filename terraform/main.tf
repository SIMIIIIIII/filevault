terraform {
  required_providers {
    docker     = { source = "kreuzwerker/docker", version = "~> 3.0" }
    kubernetes = { source = "hashicorp/kubernetes", version = "~> 3.5" }
    helm       = { source = "hashicorp/helm", version = "~> 3.3" }
    random     = { source = "hashicorp/random", version = "~> 3.0" }
  }
}

provider "docker" {
  host = var.environment == "dev" ? "unix:///var/run/docker.sock" : "null://"
}

# IMPORTANT: These resources only apply to dev environment
# Production uses k8s.tf and helm.tf instead
locals {
  deploy_docker = var.environment == "dev"
}

# Réseau - only for dev
resource "docker_network" "app" {
  count  = local.deploy_docker ? 1 : 0
  name   = "filevault-network"
  driver = "bridge"
  ipam_config {
    subnet  = "172.20.0.0/16"
    gateway = "172.20.0.1"
  }
}

# Volumes - only for dev
resource "docker_volume" "postgres_data" {
  count = local.deploy_docker ? 1 : 0
  name  = "filevault-postgres-data"
}

resource "docker_volume" "uploads" {
  count = local.deploy_docker ? 1 : 0
  name  = "filevault-uploads"
}

resource "docker_image" "filevault" {
  count         = local.deploy_docker ? 1 : 0
  name          = "${var.image_repo}:${var.image_tag}"
  pull_triggers = [var.image_tag]
  keep_locally  = true
}

# PostgreSQL - only for dev
resource "docker_container" "postgres" {
  count = local.deploy_docker ? 1 : 0
  name  = "filevault-postgres"
  image = "postgres:16-alpine"
  env = [
    "POSTGRES_DB=filevault",
    "POSTGRES_USER=filevault",
    "POSTGRES_PASSWORD=${var.db_password}",
  ]
  volumes {
    volume_name    = docker_volume.postgres_data[0].name
    container_path = "/var/lib/postgresql/data"
  }
  networks_advanced {
    name    = docker_network.app[0].name
    aliases = ["postgres"]
  }
  healthcheck {
    test     = ["CMD", "pg_isready", "-U", "filevault"]
    interval = "10s"
    timeout  = "5s"
    retries  = 5
  }
  restart = "unless-stopped"
}

# FileVault API - only for dev
resource "docker_container" "filevault" {
  count = local.deploy_docker ? 1 : 0
  name  = "filevault-api"
  image = docker_image.filevault[0].image_id
  env = [
    "DATABASE_URL=postgresql://filevault:${var.db_password}@postgres/filevault",
    "RUST_LOG=info",
  ]
  command = ["0.0.0.0", "8443"]
  ports {
    internal = 8443
    external = 8443
  }
  volumes {
    volume_name    = docker_volume.uploads[0].name
    container_path = "/data"
  }
  networks_advanced {
    name = docker_network.app[0].name
  }
  depends_on = [docker_container.postgres[0]]
  restart    = "unless-stopped"
  lifecycle {
    replace_triggered_by = [docker_image.filevault[0]]
  }
}

# Outputs
output "api_url" {
  value = "https://localhost:8443"
}