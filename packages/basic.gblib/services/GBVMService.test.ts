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


test('Compare', () => {
        
    expect(GBVMService.compare(1,1)).toBeTruthy();
    expect(GBVMService.compare({a:1},{a:1})).toBeTruthy();
    expect(GBVMService.compare({a:1},{a:2})).toBeFalsy();
    expect(GBVMService.compare({a:1, b:2},{a:1, b:2})).toBeTruthy();
   
});

test('Parse Storage Field', async () => {
    
    const s = new GBVMService();
    
    expect(await s.parseField('name STRING(30)')).toStrictEqual({name: 'name', definition: {
        allowNull: true,
        unique: false, primaryKey: false,
        size: 30,
        autoIncrement: false,
        type:"STRING"
      }});      
   
});
