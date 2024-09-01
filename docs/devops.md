# Devops

## Opening up the ports
```
sudo ufw allow proto tcp from any to any port 3000
sudo ufw allow proto tcp from any to any port 7878
```

## Ram usage
```
free -h
```

## Systemd
```bash
# Reload the systemd daemon to read the new service file
sudo systemctl daemon-reload

# Enable the service to run at startup
sudo systemctl enable plakait-backend.service

# Start the service
sudo systemctl start plakait-backend.service

# Stop service 
sudo systemctl stop plakait-backend.service

# See status
sudo systemctl status plakait-backend.service

# Restart service
sudo systemctl restart plakait-backend


# What the service's output
sudo journalctl -f -u plakait-backend.service
sudo journalctl --lines 1000 --no-pager -f -u plakait-backend.service | cut -d ' ' -f 6-
```


## htop
Use `htop`