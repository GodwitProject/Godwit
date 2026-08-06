FROM node:20-alpine AS deps
WORKDIR /app
COPY apps/ui/package*.json ./
RUN npm ci

FROM node:20-alpine AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY apps/ui/ .
# NEXT_PUBLIC_* vars are inlined into the client bundle at build time by the
# `env` block in next.config.js, so they must be supplied as ARGs here (runtime
# environment is ignored for the standalone server). Defaults mirror the
# next.config.js fallbacks. NEXT_PUBLIC_API_ORIGIN is the rewrite destination:
# same-origin auth means it should be an *internal* backend address in docker,
# never a browser-reachable host.
ARG NEXT_PUBLIC_API_ORIGIN=http://localhost:3000
ARG NEXT_PUBLIC_WS_URL=ws://localhost:3000/api/v1/ws
ENV NEXT_PUBLIC_API_ORIGIN=$NEXT_PUBLIC_API_ORIGIN
ENV NEXT_PUBLIC_WS_URL=$NEXT_PUBLIC_WS_URL
RUN mkdir -p public
RUN npm run build

FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
RUN addgroup -S nodejs && adduser -S nextjs -G nodejs
COPY --from=builder /app/next.config.js ./
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
USER nextjs
EXPOSE 3000
ENV PORT=3000
CMD ["node", "server.js"]
