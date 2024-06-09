/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance, GBService, IGBCoreService, GBLog } from 'botlib';
import * as Fs from 'fs';
import * as ji from 'just-indent'
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
import { Sequelize, QueryTypes } from '@sequelize/core';
import { z } from "zod";
import { zodToJsonSchema } from "zod-to-json-schema";

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

  public static compare(obj1, obj2) {
    //check for obj2 overlapping props
    if (!Object.keys(obj2).every(key => obj1.hasOwnProperty(key))) {
      return false;
    }

    //check every key for being same
    return Object.keys(obj1).every(function (key) {

      //if object
      if ((typeof obj1[key] == "object") && (typeof obj2[key] == "object")) {

        //recursively check
        return GBVMService.compare(obj1[key], obj2[key]);
      } else {

        //do the normal compare
        return obj1[key] === obj2[key];
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


    // TODO: #412.
    // const subscription = {
    //   changeType: 'created,updated',
    //   notificationUrl: 'https://webhook.azurewebsites.net/notificationClient',
    //   lifecycleNotificationUrl: 'https://webhook.azurewebsites.net/api/lifecycleNotifications',
    //   resource: '/me/mailfolders(\'inbox\')/messages',
    //   expirationDateTime: '2016-03-20T11:00:00.0000000Z',
    //   clientState: 'SecretClientState'
    // };

    // let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);
    
    // await client.api('/subscriptions')
    //   .post(subscription);


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

      // Pre process SET SCHEDULE calls.

      const schedules = GBVMService.getSetScheduleKeywordArgs(text);

      const s = new ScheduleServices();
      await s.deleteScheduleIfAny(min, mainName);

      let i = 1;
      await CollectionUtil.asyncForEach(schedules, async (syntax) => {

        if (s) {
            await s.createOrUpdateSchedule(min, syntax, `${mainName};${i++}`);
        }
      });

      text = text.replace(/^\s*SET SCHEDULE (.*)/gim, '');

      // Write VBS file without pragma keywords.

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
                "yaml": "2.4.2",
                "encoding": "0.1.13",
                "isomorphic-fetch": "3.0.0",
                "punycode": "2.1.1",
                "@push-rpc/core": "1.8.2",
                "@push-rpc/http": "1.8.2",
                "vm2": "3.9.11",
                "async-retry": "1.3.3"
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
        await this.translateBASIC(mainName, fullFilename, min);
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
        await this.translateBASIC(mainName, fullFilename, min);
      }
    } else {
      await this.translateBASIC(mainName, fullFilename, min);
    }

    // Syncronizes Database Objects with the ones returned from "Word".

    const tablesFile = urlJoin(folder, `${filename}.tables.json`);
    let sync = false;

    if (Fs.existsSync(tablesFile)) {
      const minBoot = GBServer.globals.minBoot;

      const tableDef = JSON.parse(Fs.readFileSync(tablesFile, 'utf8')) as any;

      const getTypeBasedOnCondition = (t, size) => {

        if (1) {
          switch (t) {
            case 'string':
              return `varchar(${size})`;
            case 'guid':
              return 'UUID';
            case 'key':
              return `varchar(${size})`;
            case 'number':
              return 'BIGINT';
            case 'integer':
              return 'INTEGER';
            case 'double':
              return 'FLOAT';
            case 'float':
              return 'FLOAT';
            case 'date':
              return 'DATE';
            case 'boolean':
              return 'BOOLEAN';
            default:
              return { type: 'TABLE', name: t };
          }

        } else {
          switch (t) {
            case 'string':
              return { key: 'STRING' };
            case 'guid':
              return { key: 'UUID' };
            case 'key':
              return { key: 'STRING' }; // Assuming key is a string data type
            case 'number':
              return { key: 'BIGINT' };
            case 'integer':
              return { key: 'INTEGER' };
            case 'double':
              return { key: 'FLOAT' };
            case 'float':
              return { key: 'FLOAT' };
            case 'date':
              return { key: 'DATE' };
            case 'boolean':
              return { key: 'BOOLEAN' };
            default:
              return { key: 'TABLE', name: t };
          }

        }
      };

      const associations = [];

      // Loads storage custom connections.

      const path = DialogKeywords.getGBAIPath(min.botId, null);
      const filePath = Path.join('work', path, 'connections.json');
      let connections = null;
      if (Fs.existsSync(filePath)) {
        connections = JSON.parse(Fs.readFileSync(filePath, 'utf8'));
      }
      const shouldSync = min.core.getParam<boolean>(
        min.instance,
        'Synchronize Database',
        false
      );

      tableDef.forEach(async t => {

        const tableName = t.name.trim();

        // Determines autorelationship.

        Object.keys(t.fields).forEach(key => {
          let obj = t.fields[key];
          obj.type = getTypeBasedOnCondition(obj.type, obj.size);
          if (obj.type.key === "TABLE") {
            obj.type.key = "BIGINT"
            associations.push({ from: tableName, to: obj.type.name });
          }
        });

        // Cutom connection for TABLE.

        const connectionName = t.connection;
        let con;

        if (connectionName && connections) {
          con = connections.filter(p => p.name === connectionName)[0];

          const dialect = con['storageDriver'];
          const host = con['storageServer'];
          const port = con['storagePort'];
          const storageName = con['storageName'];
          const username = con['storageUsername'];
          const password = con['storagePassword'];

          const logging: boolean | Function =
            GBConfigService.get('STORAGE_LOGGING') === 'true'
              ? (str: string): void => {
                GBLogEx.info(min, str);
              }
              : false;

          const encrypt: boolean = GBConfigService.get('STORAGE_ENCRYPT') === 'true';
          const acquire = parseInt(GBConfigService.get('STORAGE_ACQUIRE_TIMEOUT'));
          const sequelizeOptions = {
            define: {
              charset: 'utf8',
              collate: 'utf8_general_ci',
              freezeTableName: true,
              timestamps: false
            },
            host: host,
            port: port,
            logging: logging as boolean,
            dialect: dialect,
            quoteIdentifiers: false, // set case-insensitive
            dialectOptions: {
              options: {
                trustServerCertificate: true,
                encrypt: encrypt,
                requestTimeout: 120 * 1000
              }
            },
            pool: {
              max: 5,
              min: 0,
              idle: 10000,
              evict: 10000,
              acquire: acquire
            }
          };

          if (!min[connectionName]) {
            GBLogEx.info(min, `Loading custom connection ${connectionName}...`);
            min[connectionName] = new Sequelize(storageName, username, password, sequelizeOptions);
          }
        }

        if (!con) {
          throw new Error(`Invalid connection specified: ${connectionName}.`);
        }

        // Field checking, syncs if there is any difference.

        const seq = min[connectionName] ? min[connectionName]
          : minBoot.core.sequelize;

        if (seq) {

          const model = seq.models[tableName];
          if (model) {
            // Except Id, checks if has same number of fields.

            let equals = 0;
            Object.keys(t.fields).forEach(key => {
              let obj1 = t.fields[key];
              let obj2 = model['fieldRawAttributesMap'][key];

              if (key !== "id") {
                if (obj1 && obj2) {
                  equals++;
                }
              }

            });

            if (equals != Object.keys(t.fields).length) {
              sync = true;
            }
          }

          seq.define(tableName, t.fields);

          // New table checking, if needs sync. 
          let tables;

          if (con.storageDriver === 'mssql') {
            tables = await seq.query(`SELECT table_name, table_schema
          FROM information_schema.tables
          WHERE table_type = 'BASE TABLE'
          ORDER BY table_name ASC`, {
              type: QueryTypes.RAW
            })[0]
          }
          else if (con.storageDriver === 'mariadb') {
            tables = await seq.getQueryInterface().showAllTables();
          }

          let found = false;
          tables.forEach((storageTable) => {
            if (storageTable['table_name'] === tableName) {
              found = true;
            }
          });

          sync = sync ? sync : !found;

          associations.forEach(e => {
            const from = seq.models[e.from];
            const to = seq.models[e.to];

            try {
              to.hasMany(from);
            } catch (error) {
              throw new Error(`BASIC: Invalid relationship in ${mainName}: from ${e.from} to ${e.to} (${min.botId})... ${error.message}`);
            }

          });


          if (sync && shouldSync) {

            GBLogEx.info(min, `BASIC: Syncing changes for TABLE ${connectionName} ${tableName} keyword (${min.botId})...`);

            await seq.sync({
              alter: true,
              force: false // Keep it false due to data loss danger.
            });
            GBLogEx.info(min, `BASIC: Done sync for ${min.botId} ${connectionName} ${tableName} storage table...`);
          }
        }
      });
    }

    const parsedCode: string = Fs.readFileSync(jsfile, 'utf8');
    min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
    return filename;
  }

  public async translateBASIC(mainName, filename: any, min: GBMinInstance) {

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

    let { code, map, metadata, tasks, systemPrompt } = await this.convert(filename, mainName, basicCode);

    // Generates function JSON metadata to be used later.

    const jsonFile = `${filename}.json`;
    Fs.writeFileSync(jsonFile, JSON.stringify(metadata));

    const mapFile = `${filename}.map`;
    Fs.writeFileSync(mapFile, JSON.stringify(map));

    // Execute off-line code tasks

    await this.executeTasks(min, tasks);

    // Run JS into the GB context.

    const jsfile: string = `${filename}.js`;

    code = `
        module.exports = (async () => { 

          // Imports npm packages for this .gbdialog conversational application.

          require('isomorphic-fetch');
          const YAML = require('yaml');
          const http = require('node:http');
          const retry = require('async-retry');
          const createRpcClient = require("@push-rpc/core").createRpcClient;
          const createHttpClient = require("@push-rpc/http").createHttpClient;
          
          // Unmarshalls Local variables from server VM.

          const pid = this.pid;
          let id = this.id;
          let username = this.username;
          let mobile = this.mobile;
          let from = this.from;
          const channel = this.channel;
          const ENTER = this.ENTER;
          const headers = this.headers;
          let httpUsername = this.httpUsername;
          let httpPs = this.httpPs;
          let today = this.today;
          let now = this.now;
          let date = new Date();
          let page = null;
          const files = [];
          let col = 1;
          let index = 1;

          // Makes objects in BASIC insensitive.

          const caseInsensitive = (listOrRow) => {
            
            if (!listOrRow) {
              
              return listOrRow;
            };

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

          // Transfers auto variables into global object.

          for(__indexer in this.variables) { 
              global[__indexer] = this.variables[__indexer];
          }   


          // Defines local utility BASIC functions.

          const ubound = (gbarray) => {
            let length = 0;
            if (gbarray){
              length = gbarray.length;
              if (length > 0){
                if(gbarray[0].gbarray){
                  return length - 1;
                }
              }
            }
            return length;
          }
            
          const isarray = (gbarray) => {return Array.isArray(gbarray) };

          // Proxies remote functions as BASIC functions.
          
          const weekday = (v) => { return (async () => { return await dk.getWeekFromDate({v}) })(); };
          const hour = (v) => { return (async () => { return await dk.getHourFromDate({v}) })(); };
          const base64 =  (v) => { return (async () => { return await dk.getCoded({v}) })(); };
          const tolist =  (v) => { return (async () => { return await dk.getToLst({v}) })(); };
          const uuid =  () => { 
              var dt = new Date().getTime();
              var uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
                  var r = (dt + Math.random()*16)%16 | 0;
                  dt = Math.floor(dt/16);
                  return (c=='x' ? r :(r&0x3|0x8)).toString(16);
              });
              return uuid;
          };
          const random =  () => { return Number.parseInt((Math.random() * 8) % 8 * 100000000)};


          // Setups interprocess communication from .gbdialog run-time to the BotServer API.

          const optsRPC = {callTimeout: this.callTimeout, messageParser: data => {return JSON.parse(data)}};
          let url;
          const agent = http.Agent({ keepAlive: true });

          url = 'http://localhost:${GBVMService.API_PORT}/${min.botId}/dk';
          const dk = (await createRpcClient(() => createHttpClient(url, {agent: agent}), optsRPC)).remote;
          url = 'http://localhost:${GBVMService.API_PORT}/${min.botId}/sys';
          const sys =  (await createRpcClient(() => createHttpClient(url, {agent: agent}), optsRPC)).remote;
          url = 'http://localhost:${GBVMService.API_PORT}/${min.botId}/wa';
          const wa = (await createRpcClient(() => createHttpClient(url, {agent: agent}), optsRPC)).remote;
          url = 'http://localhost:${GBVMService.API_PORT}/${min.botId}/img';
          const img =  (await createRpcClient(() => createHttpClient(url, {agent: agent}), optsRPC)).remote;

          const timeout = (ms)=>  {
            return new Promise(resolve => setTimeout(resolve, ms));
          }

          const ensureTokens = async (firstTime) => {
            const tokens = this.tokens ? this.tokens.split(',') : [];
            
            for(__indexer in tokens) { 
              const tokenName = tokens[__indexer];
              
              // Auto update Bearar authentication for the first token.

              const expiresOn = new Date(global[tokenName + "_expiresOn"]);
              const expiration  = expiresOn.getTime() - (10 * 60 * 1000);

              // Expires token 10min. before or if it the first time, load it.

              if (expiration < new Date().getTime() || firstTime) {
                console.log ('Expired. Refreshing token...' + expiration);
                const {token, expiresOn} = await sys.getCustomToken({pid, tokenName});

                global[tokenName] = token;
                global[tokenName + "_expiresOn"]= expiresOn; 
                console.log ('DONE:' + new Date(global[tokenName + "_expiresOn"]);
              }

              if (__indexer == 0) {
                headers['Authorization'] = 'Bearer ' + global[tokenName];
              }
            }            
          };

          const TOYAML = (json) => {
             const doc = new YAML.Document();
             doc.contents = json;
             return doc.toString();
          }       

          // Line of Business logic.
          
          let __reportMerge = {adds:  0, updates: 0, skipped: 0};
          let __report = () => {
            return __reportMerge.title + ' adds: ' + __reportMerge.adds + ', updates: ' + __reportMerge.updates + ' and skipped: ' + __reportMerge.skipped + '.';
          };
          let REPORT = 'No report yet';

          try{
            await ensureTokens(true);
            ${code} 
          }
          catch(e){
            console.log(e);

            reject ({message: e.message, name: e.name});
          }
          finally{

              // Closes handles if any.

              await wa.closeHandles({pid: pid});
              await sys.closeHandles({pid: pid});

              resolve(true);
          }
        })(); 
`;

    code = ji.default(code, '  ');

    Fs.writeFileSync(jsfile, code);
    GBLogEx.info(min, `[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${code}`);

  }

  private async executeTasks(min, tasks) {
    for (let i = 0; i < tasks.length; i++) {
      const task = tasks[i];

      if (task.kind === 'writeTableDefinition') {

        // Creates an empty object that will receive Sequelize fields.

        const tablesFile = `${task.file}.tables.json`;
        Fs.writeFileSync(tablesFile, JSON.stringify(task.tables));

      }
    }
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s*|\-/gim, '').split('.')[0];
    return mainName.toLowerCase();
  }

  public static getSetScheduleKeywordArgs(code) {
    if (!code) return [];

    const lines = code.split(/\n/);
    const results = [];

    lines.forEach(line => {
      if (line.trim()) {
        console.log(line);
        const keyword = /\s*SET SCHEDULE (.*)/gi;
        let result: any = keyword.exec(line);
        if (result) {
          result = result[1].replace(/\`|\"|\'/, '')
          result = result.trim();
          results.push(result);
        }
      }
    });

    return results;
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

    text = text.replace(/\"/gm, '`');
    text = text.replace(/\¨/gm, '`');
    text = text.replace(/\“/gm, '`');
    text = text.replace(/\”/gm, '`');
    text = text.replace(/\‘/gm, "'");
    text = text.replace(/\’/gm, "'");

    return text;
  }

  public static getMetadata(mainName: string, propertiesText, description) {
    let properties = {};
    if (!propertiesText || !description) {

      return {}
    }
    const getType = asClause => {

      asClause = asClause.trim().toUpperCase();

      if (asClause.indexOf('STRING') !== -1) {
        return 'string';
      }
      else if (asClause.indexOf('OBJECT') !== -1) {
        return 'object';
      }
      else if (asClause.indexOf('INTEGER') !== -1 || asClause.indexOf('NUMBER') !== -1) {
        return 'number';
      } else {
        return 'enum';
      }
    };


    for (let i = 0; i < propertiesText.length; i++) {
      const propertiesExp = propertiesText[i];
      const t = getType(propertiesExp[2]);
      let element;

      if (t === 'enum') {
        element = z.enum(propertiesExp[2].split(','));
      } else if (t === 'string') {
        element = z.string();
      } else if (t === 'object') {
        element = z.string();
      } else if (t === 'number') {
        element = z.number();
      } else {
        GBLog.warn(`Element type invalid specified on .docx: ${propertiesExp[0]}`);
      }

      element.describe(propertiesExp[3]);
      element['type'] = t;
      properties[propertiesExp[1].trim()] = element;
    }


    let json = {
      type: "function",
      function: {
        name: `${mainName}`,
        description: description ? description : '',
        parameters: zodToJsonSchema(z.object(properties))
      }
    }

    return json;
  }

  public async parseField(line) {

    let required = line.indexOf('*') !== -1;
    let unique = /\bunique\b/gi.test(line);
    let primaryKey = /\bkey\b/gi.test(line);
    let autoIncrement = /\bauto\b/gi.test(line);

    if (primaryKey) {
      autoIncrement = true;
      unique = true;
      required = true;
    }

    line = line.replace('*', '');

    const fieldRegExp = /^\s*(\w+)\s*(\w+)(?:\((.*)\))?/gim;

    let reg = fieldRegExp.exec(line);
    const name = reg[1];
    const t = reg[2];

    let definition = {
      allowNull: !required,
      unique: unique, primaryKey: primaryKey,
      autoIncrement: autoIncrement
    };
    definition['type'] = t;

    if (reg[3]) {
      definition['size'] = Number.parseInt(reg[3] === 'max' ? '4000' : reg[3]);
    }

    return { name, definition };
  }

  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public async convert(filename: string, mainName: string, code: string) {

    // Start and End of VB2TS tags of processing.

    code = process.env.ENABLE_AUTH ? `hear GBLogExin as login\n${code}` : code;
    var lines = code.split('\n');
    const keywords = KeywordsExpressions.getKeywords();
    let current = 41;
    const map = {};
    let properties = [];
    let description;
    let table = null; // Used for TABLE keyword.
    let talk = null;
    let systemPrompt = null;
    let connection = null;
    const tasks = [];
    let fields = {};
    let tables = [];

    const outputLines = [];
    let emmitIndex = 1;
    for (let i = 1; i <= lines.length; i++) {

      let line = lines[i - 1];

      // Remove lines before statements.

      line = line.replace(/^\s*\d+\s*/gi, '');

      if (!table && !talk && !systemPrompt) {
        for (let j = 0; j < keywords.length; j++) {
          line = line.replace(keywords[j][0], keywords[j][1]); // TODO: Investigate delay here.
        }
      }

      // Pre-process "off-line" static KEYWORDS.

      let emmit = true;
      const params = /^\s*PARAM\s*(.*)\s*AS\s*(.*)\s*LIKE\s*(.*)/gim;
      const param = params.exec(line);
      if (param) {
        properties.push(param);
        emmit = false;
      }

      const descriptionKeyword = /^\s*DESCRIPTION\s(.*)/gim;
      let descriptionReg = descriptionKeyword.exec(line);
      if (descriptionReg) {
        description = descriptionReg[1];
        emmit = false;
      }

      const endSystemPromptKeyword = /^\s*END SYSTEM PROMPT\s*/gim;
      let endSystemPromptReg = endSystemPromptKeyword.exec(line);
      if (endSystemPromptReg && systemPrompt) {
        line = systemPrompt + '`})';

        systemPrompt = null;
        emmit = true;
      }

      const endTalkKeyword = /^\s*END TALK\s*/gim;
      let endTalkReg = endTalkKeyword.exec(line);
      if (endTalkReg && talk) {
        line = talk + '`})';

        talk = null;
        emmit = true;
      }

      const endTableKeyword = /^\s*END TABLE\s*/gim;
      let endTableReg = endTableKeyword.exec(line);
      if (endTableReg && table) {

        tables.push({
          name: table, fields: fields, connection: connection
        });


        fields = {};
        table = null;
        connection = null;
        emmit = false;
      }

      // Inside BEGIN TALK

      if (talk) {
        talk += line + '\\n';
        emmit = false;
      }

      // Inside BEGIN SYSTEM PROMPT

      if (systemPrompt) {
        systemPrompt += line + '\\n';
        emmit = false;
      }

      // Inside BEGIN/END table pair containing FIELDS.

      if (table && line.trim() !== '') {
        const field = await this.parseField(line);
        fields[field['name']] = field.definition;
        emmit = false;
      }

      const tableKeyword = /^\s*TABLE\s*(.*)\s*ON\s*(.*)/gim;
      let tableReg = tableKeyword.exec(line);
      if (tableReg && !table) {
        table = tableReg[1];
        connection = tableReg[2];
        emmit = false;
      }

      const talkKeyword = /^\s*BEGIN TALK\s*/gim;
      let talkReg = talkKeyword.exec(line);
      if (talkReg && !talk) {
        talk = "await dk.talk ({pid: pid, text: `";
        emmit = false;
      }

      const systemPromptKeyword = /^\s*BEGIN SYSTEM PROMPT\s*/gim;
      let systemPromptReg = systemPromptKeyword.exec(line);
      if (systemPromptReg && !systemPrompt) {
        systemPrompt = "await sys.setSystemPrompt ({pid: pid, text: `";
        emmit = false;
      }

      //  Add additional lines returned from replacement.

      let add = emmit ? line.split(/\r\n|\r|\n/).length : 0;
      current = current + (add ? add : 0);

      if (emmit) {
        emmitIndex++;
        map[emmitIndex] = current;
        outputLines[emmitIndex - 1] = line;
      }
    }

    if (tables) {
      tasks.push({
        kind: 'writeTableDefinition', file: filename, tables
      });

    }

    code = `${outputLines.join('\n')}\n`;

    let metadata = GBVMService.getMetadata(mainName, properties, description);

    return { code, map, metadata, tasks, systemPrompt };
  }

  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(
    text: string,
    min: GBMinInstance,
    step,
    pid,
    debug: boolean = false,
    params = []
  ) {
    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const sandbox = {};
    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let variables = [];

    // These variables will be automatically be available as normal BASIC variables.

    try {
      variables['aadToken'] = await (min.adminService as any)['acquireElevatedToken']
        (min.instance.instanceId, false);
    } catch (error) {
      variables['aadToken'] = 'ERROR: Configure /setupSecurity before using aadToken variable.';
    }

    // Adds all .gbot params as variables.

    const gbotConfig = JSON.parse(min.instance.params);
    let keys = Object.keys(gbotConfig);
    for (let j = 0; j < keys.length; j++) {
      const v = keys[j].replace(/\s/gi, '');
      variables[v] = gbotConfig[keys[j]];
    }

    // Auto-NLP generates BASIC variables related to entities.

    if (step ? step.context.activity.originalText : null && min['nerEngine']) {
      const result = await min['nerEngine'].process(step.context.activity.originalText);

      for (let i = 0; i < result.entities.length; i++) {
        const v = result.entities[i];
        const variableName = `${v.entity}`;
        variables[variableName] = v.option ? v.option : v.sourceText;
      }
    }

    // Adds params as variables to be added later as global objects.

    keys = Object.keys(params);
    for (let j = 0; j < keys.length; j++) {
      variables[keys[j]] = params[keys[j]];
    }

    const botId = min.botId;
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdialog`);
    const gbdialogPath = urlJoin(process.cwd(), 'work', path);
    const scriptPath = urlJoin(gbdialogPath, `${text}.js`);

    let code = min.sandBoxMap[text];
    const channel = step ? step.context.activity.channelId : 'web';


    const dk = new DialogKeywords();
    const sys = new SystemKeywords();
    await dk.setFilter({ pid: pid, value: null });


    // Find all tokens in .gbot Config.

    const strFind = ' Client ID';
    const tokens = await min.core['findParam'](min.instance, strFind);
    let tokensList = [];
    await CollectionUtil.asyncForEach(tokens, async t => {
      const tokenName = t.replace(strFind, '');
      tokensList.push(tokenName);
    });

    sandbox['tokens'] = tokensList.join(',');
    sandbox['variables'] = variables;
    sandbox['id'] = sys.getRandomId();
    sandbox['username'] = await dk.userName({ pid });
    sandbox['mobile'] = await dk.userMobile({ pid });
    sandbox['from'] = await dk.userMobile({ pid });
    sandbox['ENTER'] = String.fromCharCode(13);
    sandbox['headers'] = {};
    sandbox['httpUsername'] = '';
    sandbox['httpPs'] = '';
    sandbox['pid'] = pid;
    sandbox['contentLocale'] = contentLocale;
    sandbox['callTimeout'] = 60 * 60 * 24 * 1000;
    sandbox['channel'] = channel;
    sandbox['today'] = await dk.getToday({ pid });
    sandbox['now'] = await dk.getNow({ pid });
    sandbox['returnValue'] = null;
    let result;

    try {
      if (GBConfigService.get('GBVM') === 'false') {
        return await (async () => {
          return await new Promise((resolve, reject) => {
            sandbox['resolve'] = resolve;
            // TODO: #411 sandbox['reject'] = reject;
            sandbox['reject'] = ()=>{};

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
          });

        })();
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
    }

  }

  public static createProcessInfo(user: GuaribasUser, min: GBMinInstance, channel: any, executable: string, step = null) {
    const pid = GBAdminService.getNumberIdentifier();
    GBServer.globals.processes[pid] = {
      pid: pid,
      userId: user ? user.userId : 0,
      instanceId: min.instance.instanceId,
      channel: channel,
      roles: 'everyone',
      step: step,
      executable: executable
    };
    return pid;
  }
}
