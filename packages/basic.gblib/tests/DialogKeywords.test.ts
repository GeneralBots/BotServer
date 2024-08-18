import { expect, test } from 'vitest';
import { DialogKeywords } from '../services/DialogKeywords';
import init from '../../../.test-init'

init();

const dk = new DialogKeywords();
const pid = 1;

test('TOLIST', async () => {

  const obj = [{a:1, b:2}, {a:2, b:4}];

  expect(await dk.getToLst({ pid, array: obj, member:'a' }))
    .toBe("1,2");
});
