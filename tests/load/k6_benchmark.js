import http from 'k6/http';
import { check, group, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '10s', target: 20 },  // Ramp-up to 20 virtual users
    { duration: '30s', target: 50 },  // Sustain load at 50 VUs
    { duration: '10s', target: 0 },   // Ramp-down
  ],
  thresholds: {
    http_req_duration: ['p(95)<5', 'p(99)<10'], // 99% of requests must complete under 10ms (RNF04)
    http_req_failed: ['rate<0.001'],            // < 0.1% errors allowed
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export function setup() {
  const email = `benchmark_${Date.now()}@roubit.app`;
  const password = 'Password123!';

  // 1. Register test user
  const regPayload = JSON.stringify({ email, password });
  const regRes = http.post(`${BASE_URL}/api/v1/auth/register`, regPayload, {
    headers: { 'Content-Type': 'application/json' },
  });

  check(regRes, {
    'setup register ok': (r) => r.status === 201,
  });

  const authData = JSON.parse(regRes.body);
  const token = authData.token;

  // 2. Create sample habit
  const habitPayload = JSON.stringify({
    name: 'Daily Hydration Benchmark',
    habit_type: 'binary',
    target_value: 1.0,
    frequency_days: 127,
  });

  const habitRes = http.post(`${BASE_URL}/api/v1/habits`, habitPayload, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`,
      'Idempotency-Key': `setup-habit-${Date.now()}`,
    },
  });

  const habit = JSON.parse(habitRes.body);

  return { token, habitId: habit.id };
}

export default function (data) {
  const headers = {
    'Authorization': `Bearer ${data.token}`,
    'Content-Type': 'application/json',
  };

  group('Route A: List Habits (Fast Read)', () => {
    const res = http.get(`${BASE_URL}/api/v1/habits`, { headers });
    check(res, {
      'habits 200': (r) => r.status === 200,
    });
  });

  group('Route B: Habit Checkin (Idempotent Mutation)', () => {
    const idempotencyKey = `k6-${__VU}-${__ITER}-${Date.now()}`;
    const checkinHeaders = Object.assign({}, headers, {
      'Idempotency-Key': idempotencyKey,
    });
    const payload = JSON.stringify({
      log_date: '2026-10-30',
      actual_value: 1.0,
      is_completed: true,
    });

    const res = http.post(
      `${BASE_URL}/api/v1/habits/${data.habitId}/checkin`,
      payload,
      { headers: checkinHeaders }
    );
    check(res, {
      'checkin 200': (r) => r.status === 200,
    });
  });

  group('Route C: Journal Metadata Listing (Encrypted Zero-Knowledge Read)', () => {
    const res = http.get(`${BASE_URL}/api/v1/journal/metadata`, { headers });
    check(res, {
      'journal metadata 200': (r) => r.status === 200,
    });
  });

  sleep(0.02); // 20ms pacing
}
