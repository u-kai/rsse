#!/bin/bash
docker run --rm -d -p 8080:80 soulteary/docker-nginx-forward-proxy
#curl --proxy http://localhost:8080 https://www.google.com --include --verbose