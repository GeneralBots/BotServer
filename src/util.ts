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
VerbosityLevel.ERRORS=0;
VerbosityLevel.WARNINGS=0;
VerbosityLevel.INFOS=0;
import { Page } from 'puppeteer';
import urljoin from 'url-join';
import html2md from 'html-to-md';

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
    let config = {
      spec: JSON.parse(await fs.readFile('directline-3.0.json', 'utf8')),
      requestInterceptor: req => {
        req.headers['Authorization'] = `Bearer ${min.instance.webchatKey}`;
      }
    };
    if (!GBConfigService.get('STORAGE_NAME')) {
      (config['spec'].url = `http://127.0.0.1:${GBConfigService.getServerPort()}/api/messages/${min.botId}`),
        (config['spec'].servers = [
          { url: `http://127.0.0.1:${GBConfigService.getServerPort()}/api/messages/${min.botId}` }
        ]);
      config['spec'].openapi = '3.0.0';
      delete config['spec'].host;
      delete config['spec'].swagger;
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
    return YAML.stringify(extractedError);
  }

  public static sleep(ms) {
    return new Promise(resolve => {
      setTimeout(resolve, ms);
    });
  }

  public static caseInsensitive(listOrRow) {
    if (!listOrRow || typeof listOrRow !== 'object') {
      return listOrRow;
    }
    const lowercase = oldKey => (typeof oldKey === 'string' ? oldKey.toLowerCase() : oldKey);
    const createCaseInsensitiveProxy = obj => {
      const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));
      const caseInsensitiveGetHandler = {
        get: (target, property) => propertiesMap.get(lowercase(property))
      };
      return new Proxy(obj, caseInsensitiveGetHandler);
    };
    if (Array.isArray(listOrRow)) {
      return listOrRow.map(row => createCaseInsensitiveProxy(row));
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

 public static async savePage(url: string, page: Page, directoryPath: string): Promise<string | null> {
    try {
      // Check if the directory exists, create it if not
      const directoryExists = await this.fileExists(directoryPath);
      if (!directoryExists) {
        await fs.mkdir(directoryPath, { recursive: true }); // Create directory if it doesn't exist
      }
  
      // Check if the URL is for a downloadable file (e.g., .pdf)
      if (url.endsWith('.pdf')) {
        const response = await fetch(url);
  
        if (!response.ok) {
          throw new Error('Failed to download the file');
        }
  
        const buffer = await response.arrayBuffer(); // Convert response to array buffer
        const fileName = path.basename(url); // Extract file name from URL
        const filePath = path.join(directoryPath, fileName); // Create file path
  
        const data = new Uint8Array(buffer);
        const text = await GBUtil.getPdfText(data);

        // Write the buffer to the file asynchronously
        await fs.writeFile(filePath, text);
  
        return filePath; // Return the saved file path
      } else {
        // Use Puppeteer for non-downloadable pages

        const parsedUrl = new URL(url);

        // Get the last part of the URL path or default to 'index' if empty
        const pathParts = parsedUrl.pathname.split('/').filter(Boolean); // Remove empty parts
        const lastPath = pathParts.length > 0 ? pathParts[pathParts.length - 1] : 'index';
        const flatLastPath = lastPath.replace(/\W+/g, '-'); // Flatten the last part of the path
        
        const fileName = `${flatLastPath}.html`;
        const filePath = path.join(directoryPath, fileName);

        const htmlContent = await page.content();
  
        // Write HTML content asynchronously
        await fs.writeFile(filePath, htmlContent);
  
        return filePath;
      }
    } catch (error) {
      console.error('Error saving page:', error);
      return null;
    }
  }
  
  public static async  fileExists(filePath: string): Promise<boolean> {
    try {
      await fs.access(filePath);
      return true;
    } catch (error) {
      return false;
    }
  }
  

  public static async copyIfNewerRecursive(src, dest) {
    if (!await GBUtil.exists(src)) {
      console.error(`Source path "${src}" does not exist.`);
      return;
    }

    // Check if the source is a directory
    if ((await  fs.stat(src)).isDirectory()) {
      // Create the destination directory if it doesn't exist
      if (!await GBUtil.exists(dest)) {
        fs.mkdir(dest, { recursive: true });
      }

      // Read all files and directories in the source directory
      const entries =await  fs.readdir(src);

      for (let entry of entries) {
        const srcEntry = path.join(src, entry);
        const destEntry = path.join(dest, entry);

        // Recursively copy each entry
        this.copyIfNewerRecursive(srcEntry, destEntry);
      }
    } else {
      // Source is a file, check if we need to copy it
      if (await GBUtil.exists(dest)) {
        const srcStat =await  fs.stat(src);
        const destStat =await  fs.stat(dest);

        // Copy only if the source file is newer than the destination file
        if (srcStat.mtime > destStat.mtime) {
          fs.cp(src, dest, { force: true });
        }
      } else {
        // Destination file doesn't exist, so copy it
        fs.cp(src, dest, { force: true });
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
    
    const pdf = await getDocument({data}).promise;
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


}
