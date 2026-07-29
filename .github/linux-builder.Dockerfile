FROM ubuntu:24.04

RUN apt-get update -qq && \
    apt-get install -y -qq --no-install-recommends \
      build-essential \
      curl \
      ca-certificates \
      file \
      libssl-dev \
      libwebkit2gtk-4.1-dev \
      libayatana-appindicator3-dev \
      librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*
