FROM node:18-alpine

WORKDIR /app

# Copy package files
COPY apps/admin/package*.json ./

# Install dependencies
RUN npm install

# Copy source code
COPY apps/admin/ .

# Expose port
EXPOSE 3000

# Start dev server
CMD ["npm", "run", "dev"]
