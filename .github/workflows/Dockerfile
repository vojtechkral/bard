# The Dockerfile assumes bard is already built for the musl target

FROM alpine:3.16

RUN apk add texlive-xetex
RUN apk add texmf-dist-latexextra
RUN apk add ttf-droid

COPY --chmod=0755 target/x86_64-unknown-linux-musl/release/bard /usr/local/bin/bard

WORKDIR /workdir
