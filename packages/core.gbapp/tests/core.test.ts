import { expect } from 'chai';
import { GBImporter } from '../services/GBImporterService';

describe('Hello function', () => {
  it('should return empty test', () => {
    const service = new GBImporter(undefined);
    const result = 0;
    expect(result).to.equal(0);
  });
});
