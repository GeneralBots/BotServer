# apt install tree
tree -P '*.rs' -I 'target|*.lock' --prune | grep -v '[0-9] directories$'
