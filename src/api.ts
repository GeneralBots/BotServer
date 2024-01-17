import { ImageProcessingServices } from '../packages/basic.gblib/services/ImageProcessingServices.js';
import { SystemKeywords } from '../packages/basic.gblib/services/SystemKeywords.js';
import { WebAutomationServices } from '../packages/basic.gblib/services/WebAutomationServices.js';
import { DialogKeywords } from '../packages/basic.gblib/services/DialogKeywords.js';
import { DebuggerService } from '../packages/basic.gblib/services/DebuggerService.js';

export interface GBAPI
{
    systemKeywords: SystemKeywords;
    dialogKeywords: DialogKeywords;
    imageProcessing: ImageProcessingServices;
    webAutomation: WebAutomationServices;
    debuggerService: DebuggerService;    
}