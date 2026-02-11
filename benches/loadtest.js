// k6 Load Test Script for LogAI API
// Install: brew install k6
// Run: k6 run --vus 50 --duration 60s benches/loadtest.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const ingestLatency = new Trend('ingest_latency');
const searchLatency = new Trend('search_latency');
const askLatency = new Trend('ask_latency');

// Configuration
const BASE_URL = __ENV.API_URL || 'http://localhost:3000';

// Test options
export const options = {
  scenarios: {
    // Scenario 1: Ingestion load
    ingestion: {
      executor: 'constant-vus',
      vus: 20,
      duration: '60s',
      exec: 'ingestLogs',
      tags: { scenario: 'ingestion' },
    },
    // Scenario 2: Search load
    search: {
      executor: 'constant-vus',
      vus: 10,
      duration: '60s',
      exec: 'searchLogs',
      startTime: '10s', // Start after ingestion warms up
      tags: { scenario: 'search' },
    },
    // Scenario 3: AI queries
    ask: {
      executor: 'constant-arrival-rate',
      rate: 5, // 5 requests per second
      duration: '60s',
      preAllocatedVUs: 10,
      exec: 'askQuestion',
      startTime: '15s',
      tags: { scenario: 'ask' },
    },
  },
  thresholds: {
    'http_req_duration{scenario:ingestion}': ['p(95)<500'],
    'http_req_duration{scenario:search}': ['p(95)<200'],
    'http_req_failed': ['rate<0.01'], // Less than 1% errors
  },
};

// Sample data generators
function randomService() {
  const services = ['api-gateway', 'user-service', 'payment-service', 'order-service', 'auth-service'];
  return services[Math.floor(Math.random() * services.len)];
}

function randomLevel() {
  const levels = ['info', 'warn', 'error', 'debug'];
  const weights = [0.7, 0.15, 0.1, 0.05]; // info heavy
  const r = Math.random();
  let cumulative = 0;
  for (let i = 0; i < levels.length; i++) {
    cumulative += weights[i];
    if (r <= cumulative) return levels[i];
  }
  return 'info';
}

function randomMessage() {
  const messages = [
    'Request processed successfully',
    'Database query completed in 45ms',
    'Connection timeout after 30s',
    'User authentication failed: invalid token',
    'Payment transaction processed',
    'Cache hit for user:12345',
    'Rate limit exceeded for IP 10.0.0.1',
    'Health check passed',
    'Circuit breaker opened for redis',
    'Retrying request attempt 2/3',
    'ERROR: Out of memory exception',
    'FATAL: Cannot connect to database',
  ];
  return messages[Math.floor(Math.random() * messages.length)];
}

function generateLogBatch(size) {
  const logs = [];
  for (let i = 0; i < size; i++) {
    logs.push({
      service: randomService(),
      level: randomLevel(),
      message: `${randomMessage()} - request_id=${crypto.randomUUID()} latency=${Math.floor(Math.random() * 500)}ms`,
      timestamp: new Date().toISOString(),
      trace_id: crypto.randomUUID(),
    });
  }
  return logs;
}

// Test functions
export function ingestLogs() {
  const batch = generateLogBatch(100); // 100 logs per request
  
  const start = Date.now();
  const res = http.post(`${BASE_URL}/api/logs`, JSON.stringify(batch), {
    headers: { 'Content-Type': 'application/json' },
    tags: { name: 'POST /api/logs' },
  });
  ingestLatency.add(Date.now() - start);
  
  const success = check(res, {
    'ingestion status is 200-299': (r) => r.status >= 200 && r.status < 300,
  });
  
  errorRate.add(!success);
  sleep(0.1); // Small pause between batches
}

export function searchLogs() {
  const queries = [
    'error',
    'timeout',
    'failed',
    'connection',
    'payment error',
    'authentication',
    'database',
    'memory',
  ];
  const query = queries[Math.floor(Math.random() * queries.length)];
  
  const start = Date.now();
  const res = http.get(`${BASE_URL}/api/search?q=${encodeURIComponent(query)}&limit=50`, {
    tags: { name: 'GET /api/search' },
  });
  searchLatency.add(Date.now() - start);
  
  check(res, {
    'search status is 200': (r) => r.status === 200,
    'search returns array': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body.results || body);
      } catch {
        return false;
      }
    },
  });
  
  sleep(0.5);
}

export function askQuestion() {
  const questions = [
    'Why are there errors in the payment service?',
    'Show me all database connection failures',
    'What happened in the last hour?',
    'Summarize authentication issues today',
    'Why did users get timeout errors?',
  ];
  const question = questions[Math.floor(Math.random() * questions.length)];
  
  const start = Date.now();
  const res = http.get(`${BASE_URL}/api/ask?q=${encodeURIComponent(question)}`, {
    tags: { name: 'GET /api/ask' },
    timeout: '30s', // AI queries can be slow
  });
  askLatency.add(Date.now() - start);
  
  check(res, {
    'ask status is 200': (r) => r.status === 200,
    'ask returns answer': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.answer && body.answer.length > 0;
      } catch {
        return false;
      }
    },
  });
  
  sleep(1);
}

// Health check before tests
export function setup() {
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`API not healthy: ${res.status} - ${res.body}`);
  }
  console.log('API health check passed');
  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Test completed in ${duration.toFixed(1)}s`);
}
