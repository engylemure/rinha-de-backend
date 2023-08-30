FROM rust:1.72.0-alpine3.18

# Dockerize setup
ENV DOCKERIZE_VERSION v0.6.1
RUN wget https://github.com/jwilder/dockerize/releases/download/$DOCKERIZE_VERSION/dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && tar -C /usr/local/bin -xzvf dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz \
    && rm dockerize-linux-amd64-$DOCKERIZE_VERSION.tar.gz
    
# Adding system dependencies
RUN apk --no-cache add libpq libaio libstdc++ libc6-compat  musl musl-dev protoc protobuf-dev

# Setting up working directory
ENV HOME=/opt/app

WORKDIR $HOME

COPY api/ /opt/app/api
COPY start.sh /opt/app
COPY env.tmpl /opt/app

# Application Setup
ENTRYPOINT ["dockerize", "-template", "./env.tmpl:./api/.env"]

ENV TARGET_NAME rinha
RUN cd api && cargo build --release && cd target/release && rm -rf build deps examples incremental

# Application Execution

CMD ["sh", "./start.sh"]