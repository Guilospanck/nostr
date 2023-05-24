# apt-get update
# apt-get install -y wget build-essential checkinstall zlib1g-dev openssl pkg-config libssl-dev
# wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
# dpkg -i libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb

apt-get update
apt upgrade
apt-get install -y wget pkg-config libssl-dev musl-tools
wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb
dpkg -i libssl1.1_1.1.1f-1ubuntu2.18_amd64.deb