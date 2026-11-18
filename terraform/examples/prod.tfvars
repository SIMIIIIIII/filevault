# Exemple de configuration pour production
# Usage: terraform apply -var-file=examples/prod.tfvars
# ⚠️ CHANGE LES VALEURS AVANT D'APPLIQUER!

environment = "prod"
image_repo  = "your-registry/filevault"
image_tag   = "v1.0.0"
db_password = "CHANGE-THIS-VERY-SECURE-PASSWORD-32-CHARS-MIN"
domain      = "filevault.example.com"
replicas    = 3

tags = {
  project    = "filevault"
  environment = "production"
  managed-by = "terraform"
  critical   = "true"
}

enabled_features = ["api", "metrics", "analytics", "audit-logging"]
