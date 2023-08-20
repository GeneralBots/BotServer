/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance, GBService, IGBCoreService, GBDialogStep } from 'botlib';
import * as Fs from 'fs';
import { GBServer } from '../../../src/app.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { ScheduleServices } from './ScheduleServices.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import urlJoin from 'url-join';
import { NodeVM, VMScript } from 'vm2';
import { createVm2Pool } from './vm2-process/index.js';
import textract from 'textract';
import walkPromise from 'walk-promise';
import child_process from 'child_process';
import Path from 'path';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { DialogKeywords } from './DialogKeywords.js';
import { KeywordsExpressions } from './KeywordsExpressions.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { SystemKeywords } from './SystemKeywords.js';
import lineReplace from 'line-replace';

/**
 * @fileoverview  Decision was to priorize security(isolation) and debugging,
 * over a beautiful BASIC transpiler (to be done).
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  private static DEBUGGER_PORT = 9222;
  public static API_PORT = 1111;

  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const files = await walkPromise(folder);

    await CollectionUtil.asyncForEach(files, async file => {
      if (!file) {
        return;
      }

      let filename: string = file.name;

      if (filename.endsWith('.docx')) {
        filename = await this.loadDialog(filename, folder, min);
      }
    });
  }

  public async loadDialog(filename: string, folder: string, min: GBMinInstance) {
    const wordFile = filename;
    const vbsFile = filename.substr(0, filename.indexOf('docx')) + 'vbs';
    const fullVbsFile = urlJoin(folder, vbsFile);
    const docxStat = Fs.statSync(urlJoin(folder, wordFile));
    const interval = 3000; // If compiled is older 30 seconds, then recompile.
    let writeVBS = true;
    if (Fs.existsSync(fullVbsFile)) {
      const vbsStat = Fs.statSync(fullVbsFile);
      if (docxStat['mtimeMs'] < vbsStat['mtimeMs'] + interval) {
        writeVBS = false;
      }
    }
    filename = vbsFile;
    let mainName = GBVMService.getMethodNameFromVBSFilename(filename);
    min.scriptMap[filename] = mainName;

    if (writeVBS) {
      let text = await this.getTextFromWord(folder, wordFile);

      const schedule = GBVMService.getSetScheduleKeywordArgs(text);
      const s = new ScheduleServices();
      if (schedule) {
        await s.createOrUpdateSchedule(min, schedule, mainName);
      } else {
        await s.deleteScheduleIfAny(min, mainName);
      }
      text = text.replace(/^\s*SET SCHEDULE (.*)/gim, '');
      Fs.writeFileSync(urlJoin(folder, vbsFile), text);
    }

    // Process node_modules install.
    const node_modules = urlJoin(process.env.PWD, folder, 'node_modules');
    if (!Fs.existsSync(node_modules)) {
      const packageJson = `
            {
              "name": "${min.botId}.gbdialog",
              "version": "1.0.0",
              "description": "${min.botId} transpiled .gbdialog",
              "author": "${min.botId} owner.",
              "license": "ISC",
              "dependencies": {
                "encoding": "0.1.13",
                "isomorphic-fetch": "3.0.0",
                "punycode": "2.1.1",
                "@push-rpc/core": "1.1.5",
                "@push-rpc/http": "1.1.5",
                "vm2": "3.9.11"
              }
            }`;
      Fs.writeFileSync(urlJoin(folder, 'package.json'), packageJson);

      GBLogEx.info(min, `BASIC: Installing .gbdialog node_modules for ${min.botId}...`);
      const npmPath = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');
      child_process.execSync(`${npmPath} install`, { cwd: folder });
    }

    // Hot swap for .vbs files.
    const fullFilename = urlJoin(folder, filename);
    if (process.env.DEV_HOTSWAP) {
      Fs.watchFile(fullFilename, async () => {
        await this.translateBASIC(fullFilename, min);
        const parsedCode: string = Fs.readFileSync(jsfile, 'utf8');
        min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
      });
    }

    const compiledAt = Fs.statSync(fullFilename);
    const jsfile = urlJoin(folder, `${filename}.js`);

    if (Fs.existsSync(jsfile)) {
      const jsStat = Fs.statSync(jsfile);
      const interval = 30000; // If compiled is older 30 seconds, then recompile.
      if (compiledAt.isFile() && compiledAt['mtimeMs'] > jsStat['mtimeMs'] + interval) {
        await this.translateBASIC(fullFilename, min);
      }
    } else {
      await this.translateBASIC(fullFilename, min);
    }
    const parsedCode: string = Fs.readFileSync(jsfile, 'utf8');
    min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
    return filename;
  }

  public async translateBASIC(filename: any, min: GBMinInstance) {
    // Converts General Bots BASIC into regular VBS

    let basicCode: string = Fs.readFileSync(filename, 'utf8');

    // Process INCLUDE keyword to include another
    // dialog inside the dialog.

    let include = null;
    do {
      include = /^include\b(.*)$/gim.exec(basicCode);

      if (include) {
        let includeName = include[1].trim();
        includeName = Path.join(Path.dirname(filename), includeName);
        includeName = includeName.substr(0, includeName.lastIndexOf('.')) + '.vbs';

        // To use include, two /publish will be necessary (for now)
        // because of alphabet order may raise not found errors.

        let includeCode: string = Fs.readFileSync(includeName, 'utf8');
        basicCode = basicCode.replace(/^include\b.*$/gim, includeCode);
      }
    } while (include);

    let { code, jsonMap } = await this.convert(basicCode);
    const mapFile = `${filename}.map`;

    Fs.writeFileSync(mapFile, JSON.stringify(jsonMap));

    // Run JS into the GB context.

    const jsfile: string = `${filename}.js`;

    code = `
    return (async () => {

      // Imports npm packages for this .gbdialog conversational application.

      require('isomorphic-fetch');
      const createRpcClient = require("@push-rpc/core").createRpcClient;
      const createHttpClient = require("@push-rpc/http").createHttpClient;

      // Setups interprocess communication from .gbdialog run-time to the BotServer API.
      const optsRPC = {callTimeout: this.callTimeout};
      let url;

      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/dk';
      const dk = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/sys';
      const sys = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/wa';
      const wa = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/img';
      const img = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      
      // Unmarshalls Local variables from server VM.

      let pid = this.pid;
      let id = this.id;
      let username = this.username;
      let mobile = this.mobile;
      let from = this.from;
      let channel = this.channel;
      let ENTER = this.ENTER;
      let headers = this.headers;
      let data = this.data;
      let list = this.list;
      let httpUsername = this.httpUsername;
      let httpPs = this.httpPs;
      let today = this.today;
      let now = this.now;
      let page = null;
      let files = [];
      let col = 1;
      let index = 1 

      // Makes objects in BASIC insensitive.

      const caseInsensitive = (listOrRow) => {
        const lowercase = (oldKey) => typeof oldKey === 'string' ? oldKey.toLowerCase() : oldKey;

        const createCaseInsensitiveProxy = (obj) => {
            const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));
            const caseInsensitiveGetHandler = {
                get: (target, property) => propertiesMap.get(lowercase(property))
            };
            return new Proxy(obj, caseInsensitiveGetHandler);
        };

        if (listOrRow.length) {
            return listOrRow.map(row => createCaseInsensitiveProxy(row));
        } else {
            return createCaseInsensitiveProxy(listOrRow);
        }
      };

      // Transfers NLP auto variables into global object.

      for(i in this.variables) { 
          global[i] = this.variables[i];
      }   

      // Defines local utility BASIC functions.

      const ubound = (gbarray) => {return gbarray.length - 1};
      const isarray = (gbarray) => {return Array.isArray(gbarray) };
  
      // Proxies remote functions as BASIC functions.
      
      const weekday = (v) => { return (async () => { return await dk.getWeekFromDate({v}) })(); };
      const hour = (v) => { return (async () => { return await dk.getHourFromDate({v}) })(); };
      const base64 =  (v) => { return (async () => { return await dk.getCoded({v}) })(); };
      const tolist =  (v) => { return (async () => { return await dk.getToLst({v}) })(); };

      ${code}

      await wa.closeHandles({pid: pid});

    })(); 
`;
    Fs.writeFileSync(jsfile, code);
    GBLogEx.info(min, `[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${code}`);
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s*|\-/gim, '').split('.')[0];
    return mainName.toLowerCase();
  }

  public static getSetScheduleKeywordArgs(code: string) {
    if (!code) return null;
    const keyword = /^\s*SET SCHEDULE (.*)/gim;
    const result = keyword.exec(code);
    return result ? result[1] : null;
  }

  private async getTextFromWord(folder: string, filename: string) {
    return new Promise<string>(async (resolve, reject) => {
      const path = urlJoin(folder, filename);
      textract.fromFileWithPath(path, { preserveLineBreaks: true }, (error, text) => {
        if (error) {
          if (error.message.startsWith('File not correctly recognized as zip file')) {
            text = Fs.readFileSync(path, 'utf8');
          } else {
            reject(error);
          }
        }

        if (text) {
          text = GBVMService.normalizeQuotes(text);
        }
        resolve(text);
      });
    });
  }

  public static normalizeQuotes(text: any) {
    text = text.replace(/\¨/gm, '"');
    text = text.replace(/\“/gm, '"');
    text = text.replace(/\”/gm, '"');
    text = text.replace(/\‘/gm, "'");
    text = text.replace(/\’/gm, "'");

    return text;
  }

  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public async convert(code: string) {
    // Start and End of VB2TS tags of processing.

    code = process.env.ENABLE_AUTH ? `hear GBLogExin as login\n${code}` : code;
    var lines = code.split('\n');
    const keywords = KeywordsExpressions.getKeywords();
    let current = 41;
    const map = {}; 

    for (let i = 1; i <= lines.length; i++) {
      let line = lines[i - 1];

      // Remove lines before statments.

      line = line.replace(/^\s*\d+\s*/gi,'');

      for (let j = 0; j < keywords.length; j++) {
        line = line.replace(keywords[j][0], keywords[j][1]);
      }

      //  Add additional lines returned from replacement.

      let add = line.split(/\r\n|\r|\n/).length;
      current = current + (add ? add : 0);
      map[i] = current;
      lines[i - 1] = line;
    }

    code = `${lines.join('\n')}\n`;
    return { code, jsonMap: map };
  }

  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(
    text: string,
    min: GBMinInstance,
    step,
    user: GuaribasUser,
    deployer: GBDeployer,
    debug: boolean = false
  ) {
    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const sandbox = {};
    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // Auto-NLP generates BASIC variables related to entities.

    let variables = [];
    if (step ? step.context.activity.originalText : null && min['nerEngine']) {
      const result = await min['nerEngine'].process(step.context.activity.originalText);

      for (let i = 0; i < result.entities.length; i++) {
        const v = result.entities[i];
        const variableName = `${v.entity}`;
        variables[variableName] = v.option ? v.option : v.sourceText;
      }
    }

    const botId = min.botId;
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdialog`);
    const gbdialogPath = urlJoin(process.cwd(), 'work', path);
    const scriptPath = urlJoin(gbdialogPath, `${text}.js`);

    let code = min.sandBoxMap[text];
    const channel = step? step.context.activity.channelId : 'web';
    const pid = GBAdminService.getNumberIdentifier();
    GBServer.globals.processes[pid] = {
      pid: pid,
      userId: user? user.userId:0,
      instanceId: min.instance.instanceId,
      channel: channel
    };
    const dk = new DialogKeywords();
    const sys = new SystemKeywords();
    await dk.setFilter ({pid: pid, value: null });
    
    sandbox['variables'] = variables;
    sandbox['id'] = sys.getRandomId();
    sandbox['username'] = await dk.userName({ pid });
    sandbox['mobile'] = await dk.userMobile({ pid });
    sandbox['from'] = await dk.userMobile({ pid });
    sandbox['ENTER'] = String.fromCharCode(13);
    sandbox['headers'] = {};
    sandbox['data'] = {};
    sandbox['list'] = [];
    sandbox['httpUsername'] = '';
    sandbox['httpPs'] = '';
    sandbox['pid'] = pid;
    sandbox['contentLocale'] = contentLocale;
    sandbox['callTimeout'] =  60 * 60 * 24 * 1000;
    sandbox['channel'] = channel;
    sandbox['today'] = await dk.getToday({ pid });
    sandbox['now'] = await dk.getNow({ pid });
    let result;

    try {
      if (GBConfigService.get('GBVM') === 'false') {
        const vm1 = new NodeVM({
          allowAsync: true,
          sandbox: sandbox,
          console: 'inherit',
          wrapper: 'commonjs',
          require: {
            builtin: ['stream', 'http', 'https', 'url', 'zlib', 'net', 'tls', 'crypto'],
            root: ['./'],
            external: true,
            context: 'sandbox'
          }
        });
        const s = new VMScript(code, { filename: scriptPath });
        result = vm1.run(s);
      } else {
        const runnerPath = urlJoin(
          process.cwd(),
          'dist',
          'packages',
          'basic.gblib',
          'services',
          'vm2-process',
          'vm2ProcessRunner.js'
        );

        const { run } = createVm2Pool({
          min: 0,
          max: 0,
          debug: debug,
          debuggerport: GBVMService.DEBUGGER_PORT,
          botId: botId,
          cpu: 100,
          memory: 50000,
          time: 60 * 60 * 24 * 14,
          cwd: gbdialogPath,
          script: runnerPath
        });

        result = await run(code, { filename: scriptPath, sandbox: sandbox });
      }
    } catch (error) {
      throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
    } finally {
    }

    return result;
  }
}
