import { GBVMService } from './GBVMService';
import { expect, test } from 'vitest'

test('Default', () => {
        

    const args = GBVMService.getSetScheduleKeywordArgs(`
    
    SET SCHEDULE "0 0 */1 * * *"  
    SET SCHEDULE "0 0 */3 * * *"  
    SET SCHEDULE "0 0 */2 * * *"  
    SET SCHEDULE "0 0 */2 * * *"  
    SET SCHEDULE "0 0 */3 * * *"  
    
    `);

    expect(args.length).toBe(5);
   
});
