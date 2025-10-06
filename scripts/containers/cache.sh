curl -fsSL https://packages.redis.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/valkey.gpg
echo "deb [signed-by=/usr/share/keyrings/valkey.gpg] https://packages.redis.io/deb $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/valkey.list
sudo apt update
sudo apt install valkey-server

sudo systemctl enable valkey-server
sudo systemctl start valkey-server
