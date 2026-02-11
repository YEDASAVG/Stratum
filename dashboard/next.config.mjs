/** @type {import('next').NextConfig} */
const nextConfig = {
  // See https://lucide.dev/guide/packages/lucide-react#nextjs-example
  transpilePackages: ["lucide-react"],
  
  // Enable standalone output for Docker deployment
  output: "standalone",
}

export default nextConfig
