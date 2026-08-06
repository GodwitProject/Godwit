const API_ORIGIN = process.env.NEXT_PUBLIC_API_ORIGIN || 'http://localhost:3000';

/** @type {import('next').NextConfig} */
module.exports = {
  output: 'standalone',
  async rewrites() {
    return [
      { source: '/api/v1/:path*', destination: `${API_ORIGIN}/api/v1/:path*` },
      { source: '/health', destination: `${API_ORIGIN}/health` },
      { source: '/metrics', destination: `${API_ORIGIN}/metrics` },
      { source: '/v1/utils/:path*', destination: `${API_ORIGIN}/v1/utils/:path*` },
    ];
  },
};
