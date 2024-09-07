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

"use strict"

import { GBMinInstance, GBLog } from "botlib";
import { CollectionUtil } from 'pragmatismo-io-framework';
const MicrosoftGraph = require("@microsoft/microsoft-graph-client");
const Juno = require('juno-payment-node');
const sgMail = require('@sendgrid/mail');
const PasswordGenerator = require('strict-password-generator').default;

export class GBOService {

  public isValidCardNumber(ccNumber) {
    let card = new Juno.Card();
    return card.validateNumber(ccNumber);
  }

  public isValidSecurityCode(ccNumber, cvcNumber) {
    let card = new Juno.Card();
    return card.validateCvc(ccNumber, cvcNumber);
  }

  public isValidExpireDate(month, year) {
    let card = new Juno.Card();
    return card.validateExpireDate(month, year);
  }

  public async sendEmail(token: string, to: string, from: string,
    subject: string, text: string, html: string) {
    return new Promise<any>((resolve, reject) => {
      sgMail.setApiKey(token);
      const msg = {
        to: to,
        from: from,
        subject: subject,
        text: text,
        html: html
      };
      sgMail.send(msg, false, (err, res) => {
        if (err) {
          reject(err)
        }
        else {
          resolve(res);
        }
      });

    });
  }

  public async createSubFolderAtRoot(token: string, name: string,
    siteId: string, libraryId: string) {
    return new Promise<any>((resolve, reject) => {
      let client = MicrosoftGraph.Client.init({
        authProvider: done => {
          done(null, token);
        }
      });
      const body = {
        "name": name,
        "folder": {},
        "@microsoft.graph.conflictBehavior": "rename"
      }
      client.api(`https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root/children`)
        .post(body, (err, res) => {
          if (err) {
            reject(err)
          }
          else {
            resolve(res);
          }
        });
    });
  }
  public async createSubFolderAt(token: string, parentPath: string, name: string,
    siteId: string, libraryId: string) {
    return new Promise<any>((resolve, reject) => {
      let client = MicrosoftGraph.Client.init({
        authProvider: done => {
          done(null, token);
        }
      });
      const body = {
        "name": name,
        "folder": {},
        "@microsoft.graph.conflictBehavior": "rename"
      }
      client.api(`https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:/${parentPath}:/children`)
        .post(body, (err, res) => {
          if (err) {
            reject(err)
          }
          else {
            resolve(res);
          }
        });
    });
  }

  public async listTemplates(min: GBMinInstance) {

    let templateLibraryId = process.env.SAAS_TEMPLATE_LIBRARY;
    let siteId = process.env.STORAGE_SITE_ID;

    let token =
      await (min.adminService as any).acquireElevatedToken(min.instance.instanceId, true);

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const packagePath = `/`;
    let res = await client.api(
      `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root/children`)
      .get();

    return res.value;

  }

  public async copyTemplates(min: GBMinInstance, gbaiDest, templateName: string, kind: string, botName: string) {

    let token =
      await (min.adminService as any).acquireElevatedToken(min.instance.instanceId, true);

    let siteId = process.env.STORAGE_SITE_ID;
    let templateLibraryId = process.env.SAAS_TEMPLATE_LIBRARY;
    let libraryId = process.env.STORAGE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });

    const body =
    {
      "parentReference": { driveId: gbaiDest.parentReference.driveId, id: gbaiDest.id },
      "name": `${botName}.${kind}`
    }

    const packageName = `${templateName.split('.')[0]}.${kind}`;

    try {
      const src = await client.api(
        `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root:/${templateName}/${packageName}`)
        .get();

      return await client.api(
        `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/items/${src.id}/copy`)
        .post(body);

    } catch (error) {

      if (error.code === "itemNotFound") {

      } else if (error.code === "nameAlreadyExists") {

        let src = await client.api(
          `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root:/${templateName}/${packageName}:/children`)
          .get();
        const dstName = `${botName}.gbai/${botName}.${kind}`;
        let dst = await client.api(`https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:/${dstName}`)
          .get();

        await CollectionUtil.asyncForEach(src.value, async item => {

          const body =
          {
            "parentReference": { driveId: dst.parentReference.driveId, id: dst.id }
          }
          await client.api(
            `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/items/${item.id}/copy`)
            .post(body);
        });
      }
      else {
        GBLog.error(error);
        throw error;
      }
    }
  }

  public async createExcelFile(min: GBMinInstance, destinationFolder: any, name: string) {

    let token =
      await (min.adminService.acquireElevatedToken as any)(min.instance.instanceId, true);

    let siteId = process.env.STORAGE_SITE_ID;
    let templateLibraryId = process.env.SAAS_TEMPLATE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });

    const body =
    {
      "parentReference": { driveId: destinationFolder.parentReference.driveId, id: destinationFolder.id },
      "name": name
    }

    try {
      const src = await client.api(
        `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root:/System.gbdata/blank.xlsx`)
        .get();

      return await client.api(
        `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/items/${src.id}/copy`)
        .post(body);

    } catch (error) {

      GBLog.error(error);
      throw error;
    }
  }

  public async shareFolder(token: string, driveId: string, itemId: string, email: string) {
    return new Promise<string>((resolve, reject) => {
      let client = MicrosoftGraph.Client.init({
        authProvider: done => {
          done(null, token);
        }
      });

      const body =
      {
        "recipients": [
          {
            "email": email
          }
        ],
        "message": "General Bots Online - Packages folder",
        "requireSignIn": true,
        "sendInvitation": true,
        "roles": ["write"]
      };

      client.api(`https://graph.microsoft.com/v1.0/drives/${driveId}/items/${itemId}/invite`)
        .post(body, (err, res) => {
          if (err) {
            GBLog.error('Sharing: ' + err);
            reject(err)
          }
          else {
            resolve(res);
          }
        });
    });
  }

  public kmpSearch(pattern, text) {
    pattern = pattern.toLowerCase();
    text = text.toLowerCase();
    if (pattern.length == 0)
      return 0; // Immediate match

    // Compute longest suffix-prefix table
    var lsp = [0]; // Base case
    for (var i = 1; i < pattern.length; i++) {
      var j = lsp[i - 1]; // Start by assuming we're extending the previous LSP
      while (j > 0 && pattern.charAt(i) != pattern.charAt(j))
        j = lsp[j - 1];
      if (pattern.charAt(i) == pattern.charAt(j))
        j++;
      lsp.push(j);
    }

    // Walk through text string
    var j = 0; // Number of chars matched in pattern
    for (var i = 0; i < text.length; i++) {
      while (j > 0 && text.charAt(i) != pattern.charAt(j))
        j = lsp[j - 1]; // Fall back in the pattern
      if (text.charAt(i) == pattern.charAt(j)) {
        j++; // Next char matched, increment position
        if (j == pattern.length)
          return i - (j - 1);
      }
    }
    return -1; // Not found
  }

  public getAddressFromZipCode() {
    // https://maps.googleapis.com/maps/api/geocode/json?address=94040
  }

  /**
   * Retrives token and initialize drive client API.
   */
  public static async internalGetDriveClient(min: GBMinInstance) {
    let token = await (min.adminService as any).acquireElevatedToken(0, true);
    let siteId = process.env.STORAGE_SITE_ID;
    let libraryId = process.env.STORAGE_LIBRARY;

    let client = MicrosoftGraph.Client.init({
      authProvider: done => {
        done(null, token);
      }
    });
    const baseUrl = `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}`;
    return [baseUrl, client];
  }

  /**
   * Retrives a document from the drive, given a path and filename.
   */
  private async internalGetDocument(client: any, baseUrl: any, path: string, file: string) {
    let res = await client
      .api(`${baseUrl}/drive/root:${path}:/children`)
      .get();

    let documents = res.value.filter(m => {
      return m.name.toLowerCase() === file.toLowerCase();
    });

    if (!documents || documents.length === 0) {
      throw `File '${file}' specified on GBasic command not found. Check the .gbdata or the .gbdialog associated.`;
    }

    return documents[0];
  }
}
