az network public-ip list   --resource-group "$CLOUD_GROUP"   \
    --query "[].{Name:name, IP:ipAddress, ReverseDNS:dnsSettings.reverseFqdn}"   \
    -o table

az network public-ip update --resource-group "$CLOUD_GROUP"
 --name "pip-network-adapter-name" 
 --reverse-fqdn "outbound14.domain.com.br"

