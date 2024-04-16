#!/bin/bash

url=$(curl -s https://api.github.com/repos/codedeliveryservice/RecklessNetworks/releases/latest | \
      grep -o '"browser_download_url": *"[^"]*"' | head -n 1 | cut -d '"' -f 4)

mkdir -p ../networks
wget -O ../networks/model.nnue "$url"
