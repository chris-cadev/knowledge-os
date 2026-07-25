FROM alpine:3.22 AS user-config

RUN addgroup --system appgroup \
    && adduser --system --ingroup appgroup appuser

FROM nexdrew/mkdnsite:1.5.0

COPY --from=user-config /etc/passwd /etc/passwd
COPY --from=user-config /etc/group /etc/group

WORKDIR /site

COPY --chown=appuser:appgroup docs/ ./docs/
COPY --chown=appuser:appgroup README.md ./
COPY --chown=appuser:appgroup CONTRIBUTING.md ./
COPY --chown=appuser:appgroup CHANGELOG.md ./

USER appuser

ENTRYPOINT ["mkdnsite"]
CMD ["/site"]