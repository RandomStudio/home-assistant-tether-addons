#!/usr/bin/with-contenv bashio
HOST=$(bashio::config 'host')
PORT=$(bashio::config 'port')
USERNAME=$(bashio::config 'username')
PASSWORD=$(bashio::config 'password')

while :
do
  HOST=$HOST PORT=$PORT USERNAME=$USERNAME PASSWORD=$PASSWORD hass-tether-agent
done
