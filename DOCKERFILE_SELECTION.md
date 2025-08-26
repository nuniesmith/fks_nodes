# Dockerfile Options

This service supports two build strategies:

1. Simplified: 
   docker build -f Dockerfile.simple -t fks_node_network:simple .

2. Unified (full multi-runtime + GPU support) shared file:
   docker build -f ../../shared/shared_docker/Dockerfile -t fks_node_network:unified      --build-arg SERVICE_RUNTIME=rust .

Default repository Dockerfile may still be the unified version; adopt the simple one by renaming if desired.
