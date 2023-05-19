if ! docker info > /dev/null 2>&1; then
  echo "Docker is not running. Starting colima..."
  colima start
fi