
import { expect } from 'chai'
import 'mocha'
import {GBImporter} from '../services/GBImporter'

describe('Hello function', () => {
  
  it('should return empty test', () => {
    let service = new GBImporter(null);
    //service.importIfNotExistsBotPackage(null, null);
    const result = 0;
    expect(result).to.equal(0);
  });
});
