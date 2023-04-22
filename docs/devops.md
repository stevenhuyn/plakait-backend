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

# See status
sudo systemctl status plakait-backend.service

# Restart service
sudo systemctl restart plakait-backend
```
