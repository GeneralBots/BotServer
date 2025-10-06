sudo apt install sshfs -y
lxc init
lxc storage create default dir
lxc profile device add default root disk path=/ pool=default

sudo apt update && sudo apt install -y bridge-utils

# Enable IP forwarding
echo "[HOST] Enabling IP forwarding..."
echo "net.ipv4.ip_forward=1" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update








sudo apt purge '^nvidia-*'  # Clean existing drivers
sudo add-apt-repository ppa:graphics-drivers/ppa
sudo apt update
sudo apt install nvidia-driver-470-server  # Most stable for Kepler GPUs

wget https://developer.download.nvidia.com/compute/cuda/11.0.3/local_installers/cuda_11.0.3_450.51.06_linux.run
sudo sh cuda_11.0.3_450.51.06_linux.run --override
