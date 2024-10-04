/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots local utility.
 */
 
'use strict';
import * as YAML from 'yaml';
import SwaggerClient from 'swagger-client';
import fs from 'fs/promises';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import path from 'path';
import { VerbosityLevel, getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
VerbosityLevel.ERRORS = 0;
VerbosityLevel.WARNINGS = 0;
VerbosityLevel.INFOS = 0;
import urljoin from 'url-join';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService.js';
import { GBLogEx } from '../packages/core.gbapp/services/GBLogEx.js';
import { PngPageOutput, pdfToPng } from 'pdf-to-png-converter';
import urlJoin from 'url-join';
import { GBServer } from './app.js';

export class GBUtil {
  public static repeat(chr, count) {
    let str = '';
    for (let x = 0; x < count; x++) {
      str += chr;
    }
    return str;
  }

  public static padL(value, width, pad) {
    if (!width || width < 1) return value;

    if (!pad) pad = ' ';
    const length = width - value.length;
    if (length < 1) return value.substr(0, width);

    return (GBUtil.repeat(pad, length) + value).substr(0, width);
  }

  public static padR(value, width, pad) {
    if (!width || width < 1) return value;

    if (!pad) pad = ' ';
    const length = width - value.length;
    if (length < 1) value.substr(0, width);

    return (value + GBUtil.repeat(pad, length)).substr(0, width);
  }

  public static async getDirectLineClient(min) {
    let config;
    if (!GBConfigService.get('STORAGE_NAME')) {
      config = {
        spec: JSON.parse(await fs.readFile('directline-v2.json', 'utf8')),
        requestInterceptor: req => {
          req.headers['Authorization'] = `Bearer ${min.instance.webchatKey}`;
        }
      };
      config.spec['host'] = `127.0.0.1:${GBConfigService.getServerPort()}`;
      config.spec['basePath'] = `/api/messages/${min.botId}`;
      config.spec['schemes'] = ["http"];

    } else {
      config = {
        spec: JSON.parse(await fs.readFile('directline-v2.json', 'utf8')),
        requestInterceptor: req => {
          req.headers['Authorization'] = `Bearer ${min.instance.webchatKey}`;
        }
      };
    }
    return await new SwaggerClient(config);
  }

  public static toYAML(data) {
    const extractProps = obj => {
      return Object.getOwnPropertyNames(obj).reduce((acc, key) => {
        const value = obj[key];
        acc[key] = value && typeof value === 'object' && !Array.isArray(value) ? extractProps(value) : value;
        return acc;
      }, {});
    };
  
    const extractedError = extractProps(data);
    let yamlString = YAML.stringify(extractedError, {
      indent: 2, // Defines the indentation
      flowLevel: -1, // Forces inline formatting
      styles: { '!!null': 'canonical' } // Optional: Customize null display
    } as any);
  
    
      //yamlString = yamlString.slice(0, 256); // Truncate to 1024 bytes
    
  
    return yamlString;
  }
  
  public static sleep(ms) {
    return new Promise(resolve => {
      setTimeout(resolve, ms);
    });
  }

  public static caseInsensitive(listOrRow) {
    // If the input is not an object or array, return it as is
    if (!listOrRow || typeof listOrRow !== 'object') {
      return listOrRow;
    }

    // Helper function to convert property names to lowercase
    const lowercase = key => (typeof key === 'string' ? key.toLowerCase() : key);

    // Create a proxy that maps property accesses to lowercase property names
    const createCaseInsensitiveProxy = obj => {
      const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));

      const caseInsensitiveGetHandler = {
        get: (target, property) => propertiesMap.get(lowercase(property))
      };

      return new Proxy(obj, caseInsensitiveGetHandler);
    };

    // Handle arrays by mapping each element to a case-insensitive proxy
    if (Array.isArray(listOrRow)) {
      return listOrRow.map(row => (typeof row === 'object' && row !== null ? createCaseInsensitiveProxy(row) : row));
    } else {
      return createCaseInsensitiveProxy(listOrRow);
    }
  }

  public static async exists(filePath: string): Promise<boolean> {
    try {
      await fs.access(filePath);
      return true; // File exists
    } catch (error) {
      return false; // File does not exist
    }
  }

  public static async copyIfNewerRecursive(src, dest) {
    // Check if the source exists
    if (!(await GBUtil.exists(src))) {
      return;
    }

    // Check if the source is a directory
    if ((await fs.stat(src)).isDirectory()) {
      // Create the destination directory if it doesn't exist
      if (!(await GBUtil.exists(dest))) {
        await fs.mkdir(dest, { recursive: true });
      }

      // Read all files and directories in the source directory
      const entries = await fs.readdir(src);

      for (let entry of entries) {
        const srcEntry = path.join(src, entry);
        const destEntry = path.join(dest, entry);

        // Recursively copy each entry
        await this.copyIfNewerRecursive(srcEntry, destEntry);
      }
    } else {
      // Source is a file, check if we need to copy it
      if (await GBUtil.exists(dest)) {
        const srcStat = await fs.stat(src);
        const destStat = await fs.stat(dest);

        // Copy only if the source file is newer than the destination file
        if (srcStat.mtime > destStat.mtime) {
          await fs.cp(src, dest, { force: true });
        }
      } else {
        // Destination file doesn't exist, so copy it
        await fs.cp(src, dest, { force: true });
      }
    }
  }
  // Check if is a tree or flat object.

  public static hasSubObject(t) {
    for (var key in t) {
      if (!t.hasOwnProperty(key)) continue;
      if (typeof t[key] === 'object') return true;
    }
    return false;
  }

  public static async getPdfText(data): Promise<string> {
    const pdf = await getDocument({ data }).promise;
    let pages = [];

    for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const textContent = await page.getTextContent();
      const text = textContent.items
        .map(item => item['str'])
        .join(' ')
        .replace(/\s+/g, ' '); // Optionally remove extra spaces
      pages.push(text);
    }

    return pages.join(' ');
  }

  static getGBAIPath(botId, packageType = null, packageName = null) {
    let gbai = `${botId}.gbai`;
    if (!packageType && !packageName) {
      return GBConfigService.get('DEV_GBAI') ? GBConfigService.get('DEV_GBAI') : gbai;
    }

    if (GBConfigService.get('DEV_GBAI')) {
      gbai = GBConfigService.get('DEV_GBAI');
      botId = gbai.replace(/\.[^/.]+$/, '');
      return urljoin(GBConfigService.get('DEV_GBAI'), packageName ? packageName : `${botId}.${packageType}`);
    } else {
      return urljoin(gbai, packageName ? packageName : `${botId}.${packageType}`);
    }
  }

  public static async pdfPageAsImage(min, filename, pageNumber) {
    // Converts the PDF to PNG.
  
    GBLogEx.info(min, `Converting ${filename}, page: ${pageNumber ?? 'all'}...`);
    
    const options = {
      disableFontFace: true,
      useSystemFonts: true,
      viewportScale: 2.0,
      pagesToProcess: pageNumber !== undefined ? [pageNumber] : undefined,
      strictPagesToProcess: false,
      verbosityLevel: 0
    };
  
    const pngPages: PngPageOutput[] = await pdfToPng(filename, options);
  
    const generatedFiles = [];
  
    for (const pngPage of pngPages) {
      const buffer = pngPage.content;
      const gbaiName = GBUtil.getGBAIPath(min.botId, null);
      const localName = path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
      
      await fs.writeFile(localName, buffer, { encoding: null });
      
      generatedFiles.push({ localName: localName, url: url, data: buffer });
    }
  
    return generatedFiles.length > 0 ? generatedFiles : null;
  }

  public static async sleepRandom(min = 1, max = 5) {
    const randomDelay = Math.floor(Math.random() * (max - min + 1) + min) * 1000; 
    await new Promise(resolve => setTimeout(resolve, randomDelay));
  }  
}
