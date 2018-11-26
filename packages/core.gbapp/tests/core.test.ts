
import { expect } from 'chai';
import 'mocha';
import {GBImporter} from '../services/GBImporterService';

describe('Hello function', () => {

  it('should return empty test', () => {
    const service = new GBImporter(null);
    //service.importIfNotExistsBotPackage(null, null);
    const result = 0;
    expect(result).to.equal(0);
  });
});
