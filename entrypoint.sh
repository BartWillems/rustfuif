#!/bin/bash

set -e

echo "Starting migrations from '${PWD}'"

sqlx migrate run

echo "Finished migrating"

echo "Starting Rustfuif"

exec "$@"
