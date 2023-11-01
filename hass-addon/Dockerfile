FROM randomstudiotools/hass-tether-agent:latest

WORKDIR /app/hass-tether-agent
COPY /run.sh /app/hass-tether-agent/run.sh
RUN chmod +x /app/hass-tether-agent/run.sh
CMD [ "/app/hass-tether-agent/run.sh" ]