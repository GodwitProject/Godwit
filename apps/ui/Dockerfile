FROM node:20-alpine AS deps
WORKDIR /app
COPY apps/ui/package*.json ./
RUN npm ci

FROM node:20-alpine AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY apps/ui/ .
# next.config.js reads process.env.NEXT_PUBLIC_API_ORIGIN to build the rewrite
# destination, so it must be supplied as an ARG at build time here (the runtime
# environment is ignored for the standalone server). Defaults mirror the
# next.config.js fallback. Same-origin auth means this should be an *internal*
# backend address in docker, never a browser-reachable host.
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
