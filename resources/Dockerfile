FROM mongo:6.0

COPY seed /app/integrationos/resources/seed
WORKDIR /app/integrationos/

ENTRYPOINT ["/bin/bash", "-c", "set -u && mongorestore --host ${MONGO_HOST} --nsFrom=seed.* --nsTo=${MONGO_DATABASE}.* --authenticationDatabase admin -u ${MONGO_USERNAME} -p ${MONGO_PASSWORD} ./resources"]
