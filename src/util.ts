
/**
 * @fileoverview General Bots local utility.
 * This file contains utility functions used across the General Bots project.
 * @license AGPL-3.0
 */

'use strict';

import * as YAML from 'yaml';
import SwaggerClient from 'swagger-client';
import fs from 'fs/promises';
import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
import path from 'path';
import { VerbosityLevel, getDocument } from 'pdfjs-dist/legacy/build/pdf.mjs';
import urljoin from 'url-join';
import { GBAdminService } from '../packages/admin.gbapp/services/GBAdminService.js';
import { GBLogEx } from '../packages/core.gbapp/services/GBLogEx.js';
import { PngPageOutput, pdfToPng } from 'pdf-to-png-converter';
import urlJoin from 'url-join';
import { GBServer } from './app.js';
import { QueryTypes } from '@sequelize/core';

// ... existing code ...

/**
 * Utility class containing various helper functions for the General Bots project.
 */
export class GBUtil {
  /**
   * Repeats a character a specified number of times.
   * @param {string} chr - The character to repeat.
   * @param {number} count - The number of times to repeat the character.
   * @returns {string} The repeated string.
   */
  public static repeat(chr: string, count: number): string {
    let str = '';
    for (let x = 0; x < count; x++) {
      str += chr;
    }
    return str;
  }

  /**
   * Pads a string on the left with a specified character.
   * @param {string} value - The string to pad.
   * @param {number} width - The desired width of the padded string.
   * @param {string} [pad=' '] - The character to use for padding.
   * @returns {string} The padded string.
   */
  public static padL(value: string, width: number, pad: string = ' '): string {
    if (!width || width < 1) return value;

    if (!pad) pad = ' ';
    const length = width - value.length;
    if (length < 1) return value.substr(0, width);

    return (GBUtil.repeat(pad, length) + value).substr(0, width);
  }

  /**
   * Pads a string on the right with a specified character.
   * @param {string} value - The string to pad.
   * @param {number} width - The desired width of the padded string.
   * @param {string} [pad=' '] - The character to use for padding.
   * @returns {string} The padded string.
   */
  public static padR(value: string, width: number, pad: string = ' '): string {
    if (!width || width < 1) return value;

    if (!pad) pad = ' ';
    const length = width - value.length;
    if (length < 1) value.substr(0, width);

    return (value + GBUtil.repeat(pad, length)).substr(0, width);
  }

  /**
   * Gets a DirectLine client for bot communication.
   * @param {any} min - The minimum configuration object.
   * @returns {Promise<SwaggerClient>} A promise that resolves to a SwaggerClient instance.
   */
  public static async getDirectLineClient(min: any): Promise<SwaggerClient> {
    let config;
    if (GBConfigService.get('GB_MODE') !== 'legacy') {
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

  /**
   * Converts data to YAML format.
   * @param {any} data - The data to convert to YAML.
   * @returns {string} The YAML representation of the data.
   */
  public static toYAML(data: any): string {
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

  /**
   * Implements a delay function.
   * @param {number} ms - The number of milliseconds to sleep.
   * @returns {Promise<void>} A promise that resolves after the specified delay.
   */
  public static sleep(ms: number): Promise<void> {
    return new Promise(resolve => {
      setTimeout(resolve, ms);
    });
  }

  /**
   * Creates case-insensitive proxies for objects or arrays.
   * @param {any} listOrRow - The object or array to make case-insensitive.
   * @returns {any} A case-insensitive version of the input.
   */
  public static caseInsensitive(listOrRow: any): any {
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

  /**
   * Checks if a file exists.
   * @param {string} filePath - The path of the file to check.
   * @returns {Promise<boolean>} A promise that resolves to true if the file exists, false otherwise.
   */
  public static async exists(filePath: string): Promise<boolean> {
    try {
      await fs.access(filePath);
      return true; // File exists
    } catch (error) {
      return false; // File does not exist
    }
  }

  /**
   * Recursively copies files if they are newer.
   * @param {string} src - The source path.
   * @param {string} dest - The destination path.
   * @returns {Promise<void>} A promise that resolves when the copy operation is complete.
   */
  public static async copyIfNewerRecursive(src: string, dest: string): Promise<void> {
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

  /**
   * Lists database tables.
   * @param {any} dialect - The database dialect.
   * @param {any} seq - The Sequelize instance.
   * @returns {Promise<string[]>} A promise that resolves to an array of table names.
   */
  public static async listTables(dialect: any, seq: any): Promise<string[]> {
    let tables;
    if (dialect === 'sqlite') {
      tables = await seq.getQueryInterface().showAllTables();
    } else {
      // Extracting table name from the object returned by MSSQL
      tables = await seq.getQueryInterface().showAllTables();
      tables = tables.map((table: any) => table.tableName); // Extracting the table name
    }
    return tables;
  }

  /**
   * Checks if an object has sub-objects.
   * @param {any} t - The object to check.
   * @returns {boolean} True if the object has sub-objects, false otherwise.
   */
  public static hasSubObject(t: any): boolean {
    for (var key in t) {
      if (!t.hasOwnProperty(key)) continue;
      if (typeof t[key] === 'object') return true;
    }
    return false;
  }

  /**
   * Extracts text from a PDF.
   * @param {any} data - The PDF data.
   * @returns {Promise<string>} A promise that resolves to the extracted text.
   */
  public static async getPdfText(data: any): Promise<string> {
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

  /**
   * Gets the path for GBAI (General Bots AI) files.
   * @param {string} botId - The bot ID.
   * @param {string} [packageType] - The package type.
   * @param {string} [packageName] - The package name.
   * @returns {string} The GBAI path.
   */
  static getGBAIPath(botId: string, packageType?: string, packageName?: string): string {
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

  /**
   * Converts a PDF page to an image.
   * @param {any} min - The minimum configuration object.
   * @param {string} filename - The filename of the PDF.
   * @param {number} [pageNumber] - The page number to convert (optional).
   * @returns {Promise<any[]>} A promise that resolves to an array of generated image files.
   */
  public static async pdfPageAsImage(min: any, filename: string, pageNumber?: number): Promise<any[]> {
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

  /**
   * Implements a random delay.
   * @param {number} [min=1] - The minimum delay in seconds.
   * @param {number} [max=5] - The maximum delay in seconds.
   * @returns {Promise<void>} A promise that resolves after the random delay.
   */
  public static async sleepRandom(min: number = 1, max: number = 5): Promise<void> {
    const randomDelay = Math.floor(Math.random() * (max - min + 1) + min) * 1000;
    await new Promise(resolve => setTimeout(resolve, randomDelay));
  }

  public static isContentPage(text: string): boolean {
    // Common patterns that indicate non-content pages
    const nonContentPatterns = [
      /^index$/i,
      /^table of contents$/i,
    ];
  
    // Check if page is mostly dots, numbers or blank
    const isDotLeaderPage = text.replace(/\s+/g, '').match(/\.{10,}/);
    const isNumbersPage = text.replace(/\s+/g, '').match(/^\d+$/);
    const isBlankPage = text.trim().length === 0;
  
    // Check if page has actual content
    const wordCount = text.trim().split(/\s+/).length;
    const hasMinimalContent = wordCount > 10;
  
    // Check if page matches any non-content patterns
    const isNonContent = nonContentPatterns.some(pattern => 
      pattern.test(text.trim())
    );
  
    // Page is valid content if:
    // - Not mostly dots/numbers/blank
    // - Has minimal word count
    // - Doesn't match non-content patterns
    return !isDotLeaderPage && 
           !isNumbersPage && 
           !isBlankPage &&
           hasMinimalContent &&
           !isNonContent;
  }
  


}
