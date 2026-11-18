# Configuration des GitHub Actions Secrets

## 📋 Secrets requis

Pour que les workflows GitHub Actions fonctionnent correctement, vous devez configurer les secrets suivants:

### 1. **KUBECONFIG** (obligatoire pour le déploiement prod)
Configuration de votre cluster Kubernetes encodée en base64.

#### Obtenir votre KUBECONFIG:

**Pour k3s local:**
```bash
cat /etc/rancher/k3s/k3s.yaml | base64 -w 0
```

**Pour Azure AKS:**
```bash
# Se connecter à Azure
az login

# Obtenir les credentials du cluster
az aks get-credentials --resource-group <RESOURCE_GROUP> --name <CLUSTER_NAME>

# Encoder la config
cat ~/.kube/config | base64 -w 0
```

**Pour Docker Desktop Kubernetes:**
```bash
cat ~/.kube/config | base64 -w 0
```

**Pour Minikube:**
```bash
cat ~/.minikube/profiles/minikube/client.crt | base64 -w 0
# puis ~/.minikube/profiles/minikube/client.key
# et ~/.minikube/ca.crt
# Les combiner dans un seul kubeconfig
```

### 2. **PROD_DOMAIN** (obligatoire pour le déploiement prod)
Votre domaine de production (ex: `filevault.example.com`)

### 3. **PROD_DB_PASSWORD** (obligatoire pour le déploiement prod)
Mot de passe sécurisé pour la base de données PostgreSQL en production.

Générez un mot de passe fort:
```bash
openssl rand -base64 32
```

### 4. **REGISTRY_URL** (obligatoire pour le déploiement prod)
L'URL de votre registre Docker (ex: `docker.io/username` ou `myregistry.azurecr.io`)

### 5. **DOCKER_USERNAME** (pour les tests de déploiement)
Votre nom d'utilisateur Docker Hub ou registre privé

### 6. **DOCKER_TOKEN** (pour les tests de déploiement)
Votre token d'accès Docker (créé dans Docker Hub ou votre registre)

### 7. **SLACK_WEBHOOK** (optionnel)
URL webhook Slack pour les notifications de déploiement.

Pour créer un webhook Slack:
1. Allez sur https://api.slack.com/apps
2. Créez une nouvelle app
3. Activez "Incoming Webhooks"
4. Créez un nouveau webhook pour votre canal

## 🔧 Comment ajouter les secrets sur GitHub

1. Allez sur votre repository GitHub
2. Cliquez sur **Settings** → **Secrets and variables** → **Actions**
3. Cliquez sur **New repository secret**
4. Entrez le nom du secret et sa valeur
5. Cliquez sur **Add secret**

## ✅ Vérifier la configuration

Une fois configurés, les secrets sont automatiquement utilisés par les workflows GitHub Actions.

Pour vérifier que les secrets sont correctement configurés:
1. Allez sur **Actions**
2. Sélectionnez le workflow **Terraform CI/CD**
3. Lancez le workflow manuellement en cliquant sur **Run workflow**
4. Regardez les logs pour voir si les secrets sont détectés

## 🚀 Pour démarrer sans un cluster Kubernetes

Si vous n'avez pas de cluster Kubernetes en production:

**Option 1:** Créer un cluster Azure AKS gratuit
```bash
# Créer un resource group
az group create --name myResourceGroup --location eastus

# Créer un cluster AKS
az aks create --resource-group myResourceGroup \
  --name myCluster \
  --node-count 1 \
  --vm-set-type VirtualMachineScaleSets \
  --load-balancer-sku standard \
  --enable-managed-identity

# Obtenir les credentials
az aks get-credentials --resource-group myResourceGroup --name myCluster
```

**Option 2:** Créer un cluster local avec kind
```bash
# Installer kind (Kubernetes in Docker)
# https://kind.sigs.k8s.io/docs/user/quick-start/

kind create cluster
cat ~/.kube/config | base64 -w 0
```

## 📝 Notes de sécurité

- ⚠️ **Ne commitez JAMAIS les secrets dans Git**
- 🔒 Les secrets GitHub sont chiffrés et ne sont visibles que dans les logs (si nécessaire)
- 🛡️ Les secrets ne sont jamais affichés dans les logs GitHub Actions par défaut
- 🔑 Utilisez des token à courte durée de vie quand possible
- 🚫 Révoquez les secrets compromis immédiatement

## 🆘 Dépannage

**Le workflow échoue avec "KUBECONFIG is missing":**
- Vérifiez que vous avez ajouté le secret `KUBECONFIG`
- Vérifiez que la valeur est correctement encodée en base64

**Le Terraform apply échoue avec "Kubernetes cluster unreachable":**
- Vérifiez que votre cluster Kubernetes est accessible
- Vérifiez que le kubeconfig encodé est valide
- Essayez de décoder et de tester localement:
  ```bash
  echo "$KUBECONFIG_VALUE" | base64 -d > /tmp/kube.yaml
  kubectl --kubeconfig /tmp/kube.yaml get nodes
  ```

**Les notifications Slack ne sont pas envoyées:**
- Ce n'est pas un problème - le webhook Slack est optionnel
- Si vous voulez les notifications, configurez le secret `SLACK_WEBHOOK`
