FROM node:18.17-alpine3.18 as buildbase

ENV DOCKERIZE_VERSION v0.6.1
RUN wget https://github.com/jwilder/dockerize/releases/download/$DOCKERIZE_VERSION/dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && tar -C /usr/local/bin -xzvf dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && rm dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz

# Adding system dependencies
RUN apk --no-cache add libpq libaio libstdc++ libc6-compat  musl musl-dev protoc protobuf-dev

# Setting up working directory
ENV HOME=/opt/app

WORKDIR $HOME

COPY node_api/ /opt/app/api
COPY node_api/start.sh /opt/app
COPY env.tmpl /opt/app

# Application Setup
ENTRYPOINT ["dockerize", "-template", "./env.tmpl:./api/.env"]

ENV TARGET_NAME rinha
RUN cd api && npm ci

# Application Execution

CMD ["sh", "./start.sh"]