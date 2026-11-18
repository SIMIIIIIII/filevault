# Exemple de configuration pour staging
# Usage: terraform apply -var-file=examples/staging.tfvars

environment = "staging"
image_repo  = "your-registry/filevault"
image_tag   = "v1.0.0-staging"
db_password = "staging-secure-password-change-me"
domain      = "staging-filevault.example.com"
replicas    = 2

tags = {
  project     = "filevault"
  environment = "staging"
  managed-by  = "terraform"
}

enabled_features = ["api", "metrics", "analytics"]
