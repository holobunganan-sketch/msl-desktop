import { test } from 'node:test';
import assert from 'node:assert/strict';
import { get } from 'svelte/store';
import { addToast, dismissToast, toasts } from '../src/lib/stores/toast.ts';

test('notification rail shows one current message and can dismiss every kind', () => {
  toasts.set([]);
  addToast('Earlier failure', 'error', 0);
  const latest = addToast('Latest result', 'success', 0);
  assert.equal(get(toasts).length, 1);
  assert.equal(get(toasts)[0].message, 'Latest result');
  assert.equal(get(toasts)[0].dismissible, true);
  dismissToast(latest);
  assert.equal(get(toasts).length, 0);
});

test('an earlier timeout cannot dismiss a newer notification', async () => {
  toasts.set([]);
  addToast('Old', 'info', 10);
  const latest = addToast('New', 'error', 0);
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.equal(get(toasts)[0].id, latest);
  dismissToast(latest);
});
