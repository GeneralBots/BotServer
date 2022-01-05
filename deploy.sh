#!/bin/bash

# ------------------------
# General Bots deployment.
# ------------------------

# Helpers
# -------

exitWithMessageOnError () {
  if [ ! $? -eq 0 ]; then
    echo "[General Bots Deployer]An error has occurred during web site deployment."
    echo $1
    exit 1
  fi
}

# Prerequisites
# -------------

# Verify node.js installed
hash node 2>/dev/null
exitWithMessageOnError "Missing node.js executable, please install node.js, if already installed make sure it can be reached from current environment."

# Setup
# -----

SCRIPT_DIR="${BASH_SOURCE[0]%\\*}"
SCRIPT_DIR="${SCRIPT_DIR%/*}"
ARTIFACTS=$SCRIPT_DIR/../artifacts
KUDU_SYNC_CMD=${KUDU_SYNC_CMD//\"}

if [[ ! -n "$DEPLOYMENT_SOURCE" ]]; then
  DEPLOYMENT_SOURCE=$SCRIPT_DIR
fi

if [[ ! -n "$NEXT_MANIFEST_PATH" ]]; then
  NEXT_MANIFEST_PATH=$ARTIFACTS/manifest

  if [[ ! -n "$PREVIOUS_MANIFEST_PATH" ]]; then
    PREVIOUS_MANIFEST_PATH=$NEXT_MANIFEST_PATH
  fi
fi

if [[ ! -n "$DEPLOYMENT_TARGET" ]]; then
  DEPLOYMENT_TARGET=$ARTIFACTS/wwwroot
else
  KUDU_SERVICE=true
fi

if [[ ! -n "$KUDU_SYNC_CMD" ]]; then
  # Install kudu sync
  echo Installing Kudu Sync
  npm install kudusync -g --silent
  exitWithMessageOnError "npm failed"

  if [[ ! -n "$KUDU_SERVICE" ]]; then
    # In case we are running locally this is the correct location of kuduSync
    KUDU_SYNC_CMD=kuduSync
  else
    # In case we are running on kudu service this is the correct location of kuduSync
    KUDU_SYNC_CMD=$APPDATA/npm/node_modules/kuduSync/bin/kuduSync
  fi
fi

##################################################################################################################################
# Deployment
# ----------

# 1. Install npm packages
if [ -e "$DEPLOYMENT_SOURCE/package.json" ]; then
  echo "[General Bots Deployer] Running npm install..."
  cd "$DEPLOYMENT_SOURCE"
  eval npm install
  echo "[General Bots Deployer] OK."
  exitWithMessageOnError "npm failed"
 cd - > /dev/null
fi

# 2. Install TypeScript
echo "[General Bots Deployer] Transpiling..." 
eval ./node_modules/typescript/bin/tsc -v
eval ./node_modules/typescript/bin/tsc -p "$DEPLOYMENT_SOURCE"

# 3. Install default.gbui npm packages
if [ -e "$DEPLOYMENT_SOURCE/packages/default.gbui/package.json" ]; then
  echo "[General Bots Deployer] Running npm install for default.gbui..."
  cd "$DEPLOYMENT_SOURCE/packages/default.gbui"
  eval npm install
  exitWithMessageOnError "npm failed"
  echo "[General Bots Deployer] Building react app..."
  eval npm run build
  cd ..
  echo "[General Bots Deployer] OK."
  exitWithMessageOnError "react build failed"
 cd - > /dev/null
fi

echo "[General Bots Deployer] Deployment Finished."

# 4. KuduSync
if [[ "$IN_PLACE_DEPLOYMENT" -ne "1" ]]; then
  "$KUDU_SYNC_CMD" -v 50 -f "$DEPLOYMENT_SOURCE" -t "$DEPLOYMENT_TARGET" -n "$NEXT_MANIFEST_PATH" -p "$PREVIOUS_MANIFEST_PATH" -i ".git;.hg;.deployment;deploy.sh"
  exitWithMessageOnError "Kudu Sync failed"
fi

##################################################################################################################################
echo "[General Bots Deployer]Finished successfully."  