import { GBVMService } from '../services/GBVMService';
import { expect, test } from 'vitest';
import { SystemKeywords } from '../services/SystemKeywords';

const s = new SystemKeywords();
const pid = 1;

test('APPEND', async () => {
  expect(await s.append({ pid, args: [1, 1, 1, 1] })).toStrictEqual([1, 1, 1, 1]);
  expect(await s.append({ pid, args: [1] })).toStrictEqual([1]);
  expect(await s.append({ pid, args: [] })).toStrictEqual([]);
  expect(await s.append({ pid, args: null })).toStrictEqual([]);
});

test('COMPARE', () => {
  expect(GBVMService.compare(1, 1)).toBeTruthy();
  expect(GBVMService.compare({ a: 1 }, { a: 1 })).toBeTruthy();
  expect(GBVMService.compare({ a: 1 }, { a: 2 })).toBeFalsy();
  expect(GBVMService.compare({ a: 1, b: 2 }, { a: 1, b: 2 })).toBeTruthy();
});

test('Parse Storage Field', async () => {
  const s = new GBVMService();

  expect(await s.parseField('name STRING(30)')).toStrictEqual({
    name: 'name',
    definition: {
      allowNull: true,
      unique: false,
      primaryKey: false,
      size: 30,
      autoIncrement: false,
      type: 'STRING'
    }
  });
});
