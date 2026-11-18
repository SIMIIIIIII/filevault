# Namespace - only for K8s environments (staging, prod)
resource "kubernetes_namespace_v1" "filevault" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name   = "filevault"
    labels = local.common_labels
  }
}

# PersistentVolumeClaim for uploads - only for K8s
resource "kubernetes_persistent_volume_claim_v1" "uploads" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-uploads"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
  }
  spec {
    access_modes       = ["ReadWriteOnce"]
    storage_class_name = "local-path"
    resources {
      requests = {
        storage = "10Gi"
      }
    }
  }
}

# Secret TLS
resource "kubernetes_secret_v1" "tls" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-tls"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
  }
  type = "kubernetes.io/tls"
  data = {
    "tls.crt" = file("../pki/server/server.crt")
    "tls.key" = file("../pki/server/server.key")
  }
}

# Secret applicatif 
resource "random_password" "db" {
  count   = var.environment != "dev" ? 1 : 0
  length  = 32
  special = false
}

resource "random_password" "jwt" {
  count   = var.environment != "dev" ? 1 : 0
  length  = 64
  special = true
}

resource "kubernetes_secret_v1" "app" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-secrets"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
  }
  data = {
    "database-url" = "postgresql://filevault:${random_password.db[0].result}@postgres/filevault"
    "jwt-secret"   = random_password.jwt[0].result
  }
}

# ConfigMap
resource "kubernetes_config_map_v1" "config" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-config"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
  }
  data = {
    "log-level"       = var.environment == "prod" ? "info" : "debug"
    "max-upload-size" = "5368709120"
  }
}

# Deployment
resource "kubernetes_deployment_v1" "filevault" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-api"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
    labels    = local.common_labels
  }
  spec {
    replicas = local.replicas
    selector {
      match_labels = { "app" = "filevault" }
    }
    template {
      metadata {
        labels = merge(local.common_labels, { "app" = "filevault" })
      }
      spec {
        container {
          name  = "filevault"
          image = "${var.image_repo}:${var.image_tag}"
          port { container_port = 8443 }
          env {
            name = "DATABASE_URL"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.app[0].metadata[0].name
                key  = "database-url"
              }
            }
          }
          resources {
            requests = { memory = "128Mi", cpu = "100m" }
            limits   = { memory = "512Mi", cpu = "1000m" }
          }
          liveness_probe {
            http_get {
              path = "/health"
              port = 8080
            }
            initial_delay_seconds = 15
            period_seconds        = 10
          }
        }
      }
    }
  }
}

# Service
resource "kubernetes_service_v1" "filevault" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-api"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
  }
  spec {
    selector = { "app" = "filevault" }
    type     = "ClusterIP"
    port {
      port        = 8443
      target_port = 8443
    }
  }
}

# Ingress
resource "kubernetes_ingress_v1" "filevault" {
  count = var.environment != "dev" ? 1 : 0
  metadata {
    name      = "filevault-ingress"
    namespace = kubernetes_namespace_v1.filevault[0].metadata[0].name
    annotations = {
      "nginx.ingress.kubernetes.io/proxy-body-size" = "5g"
    }
  }
  spec {
    ingress_class_name = "nginx"
    tls {
      hosts       = [var.domain]
      secret_name = kubernetes_secret_v1.tls[0].metadata[0].name
    }
    rule {
      host = var.domain
      http {
        path {
          path      = "/"
          path_type = "Prefix"
          backend {
            service {
              name = kubernetes_service_v1.filevault[0].metadata[0].name
              port { number = 8443 }
            }
          }
        }
      }
    }
  }
}