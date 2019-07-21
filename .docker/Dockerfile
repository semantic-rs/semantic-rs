FROM rust:latest

# REQUIRED ENV VARS

# git repository url
ARG REPO_URL

# git repository branch
ARG REPO_BRANCH

# build command to run
ARG BUILD_CMD

# command to execute
ARG EXEC_CMD
ENV EXEC=$EXEC_CMD

# OPTIONAL ENV VARS

# relative path to the binary file (would be copied to /bin/)
ARG BINARY_PATH=""

# whether to cleanup the build
ARG CLEANUP=false

# Install dependencies
RUN apt-get update && apt-get install -y git cmake

# Create user and set workdir
RUN useradd -ms /bin/bash semantic
USER semantic
WORKDIR /home/semantic/

# Clone the repo
RUN git clone --recursive $REPO_URL project
WORKDIR /home/semantic/project

# Checkout branch
RUN git checkout $REPO_BRANCH

# Run build command
RUN bash -c "$BUILD_CMD"

# Change back to root user
USER root

# Optionally copy binary to /bin/
RUN if [ -f $BINARY_PATH ]; then cp $BINARY_PATH /bin/; fi

# Optional cleanup
RUN if $CLEANUP; then rm -rf /home/semantic/project; fi

# Switch back to user semantic and set a workdir for a volume mountpoint
USER semantic
RUN mkdir /home/semantic/workdir
WORKDIR /home/semantic/workdir

# Run cmd
CMD $EXEC
