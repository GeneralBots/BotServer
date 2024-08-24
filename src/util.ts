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
import Fs from 'fs';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import path from 'path';

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
      spec: JSON.parse(Fs.readFileSync('directline-3.0.json', 'utf8')),
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

  public static copyIfNewerRecursive(src, dest) {
    if (!Fs.existsSync(src)) {
      console.error(`Source path "${src}" does not exist.`);
      return;
    }

    // Check if the source is a directory
    if (Fs.statSync(src).isDirectory()) {
      // Create the destination directory if it doesn't exist
      if (!Fs.existsSync(dest)) {
        Fs.mkdirSync(dest, { recursive: true });
      }

      // Read all files and directories in the source directory
      const entries = Fs.readdirSync(src);

      for (let entry of entries) {
        const srcEntry = path.join(src, entry);
        const destEntry = path.join(dest, entry);

        // Recursively copy each entry
        this.copyIfNewerRecursive(srcEntry, destEntry);
      }
    } else {
      // Source is a file, check if we need to copy it
      if (Fs.existsSync(dest)) {
        const srcStat = Fs.statSync(src);
        const destStat = Fs.statSync(dest);

        // Copy only if the source file is newer than the destination file
        if (srcStat.mtime > destStat.mtime) {
          Fs.cpSync(src, dest, { force: true });
        }
      } else {
        // Destination file doesn't exist, so copy it
        Fs.cpSync(src, dest, { force: true });
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
}
