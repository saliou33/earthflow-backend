#!/bin/bash
set -e

# Load environment variables if .env exists (though Docker usually handles this via ENV or -e)
# if [ -f .env ]; then
#   export $(echo $(cat .env | sed 's/#.*//g' | xargs) | envsubst)
# fi

echo "Waiting for database to be ready..."
# Use a simple loop to wait for the database if DATABASE_URL is set
if [ -n "$DATABASE_URL" ]; then
  # Extract host and port from DATABASE_URL (very simple parsing)
  DB_HOST=$(echo $DATABASE_URL | sed -e 's|postgres://.*@||' -e 's|:.*||' -e 's|/.*||')
  DB_PORT=$(echo $DATABASE_URL | sed -e 's|postgres://.*@.*:||' -e 's|/.*||')
  
  if [ -z "$DB_PORT" ]; then DB_PORT=5432; fi
  
  while ! nc -z $DB_HOST $DB_PORT; do
    echo "Database ($DB_HOST:$DB_PORT) is not reachable yet. Retrying in 2 seconds..."
    sleep 2
  done
  echo "Database is up!"
fi

# Run migrations if migrations directory exists
if [ -d "./migrations" ]; then
  echo "Running database migrations..."
  # Ensure the database exists
  /usr/local/bin/sqlx database create
  
  # Run migrations
  /usr/local/bin/sqlx migrate run
fi

echo "Starting EarthFlow Backend..."
exec /app/backend
