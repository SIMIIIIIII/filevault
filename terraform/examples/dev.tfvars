# Exemple de configuration pour développement local
# Usage: terraform apply -var-file=examples/dev.tfvars

environment = "dev"
image_repo  = "filevault"
image_tag   = "latest"
db_password = "dev-password-123-change-me"
domain      = "filevault.local"
replicas    = 1

tags = {
  project     = "filevault"
  environment = "development"
  managed-by  = "terraform"
}

enabled_features = ["api", "metrics"]
