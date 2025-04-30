// General Bots Copyright (c) pragmatismo.com.br. All rights reserved. Licensed under the AGPL-3.0.

"use strict"

import { GBMinInstance, GBLog } from "botlib";
import { CollectionUtil } from 'pragmatismo-io-framework';
import MicrosoftGraph from "@microsoft/microsoft-graph-client";

import sgMail from '@sendgrid/mail';
import { default as PasswordGenerator } from 'strict-password-generator';
import { promises as fs } from 'fs';
import { existsSync } from 'fs';

import { GBConfigService } from "../../core.gbapp/services/GBConfigService.js";
import path from "path";
import { Client } from "minio";
import { GBLogEx } from "packages/core.gbapp/services/GBLogEx.js";

export class GBOService {

  public async listTemplates(min: GBMinInstance) {
    if (GBConfigService.get('GB_MODE') === 'legacy') {
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
    else {

      const templatesDir = path.join(process.env.PWD, 'templates');
      const gbaiDirectories = [];

      // Read all entries in the templates directory
      const entries = await fs.readdir(templatesDir, { withFileTypes: true });

      for (const entry of entries) {
        // Check if it's a directory and ends with .gbai
        if (entry.isDirectory() && entry.name.endsWith('.gbai')) {
          gbaiDirectories.push({ name: entry.name });
        }
      }
      return gbaiDirectories;
    }
  }
  public async copyTemplates(min: GBMinInstance, gbaiDest: any, templateName: string, kind: string, botName: string): Promise<void> {
    const storageMode = process.env.GB_MODE;

    if (storageMode === 'legacy') {
      // Microsoft Graph (SharePoint) Implementation
      const token = await (min.adminService as any).acquireElevatedToken(min.instance.instanceId, true);
      const siteId = process.env.STORAGE_SITE_ID;
      const templateLibraryId = process.env.SAAS_TEMPLATE_LIBRARY;
      const libraryId = process.env.STORAGE_LIBRARY;

      const client = MicrosoftGraph.Client.init({
        authProvider: (done) => done(null, token)
      });

      const packageName = `${templateName.split('.')[0]}.${kind}`;
      const destinationName = `${botName}.${kind}`;

      try {
        // Try direct copy first
        const src = await client.api(
          `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root:/${templateName}/${packageName}`
        ).get();

        await client.api(
          `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/items/${src.id}/copy`
        ).post({
          parentReference: {
            driveId: gbaiDest.parentReference.driveId,
            id: gbaiDest.id
          },
          name: destinationName
        });
      } catch (error) {
        if (error.code === "nameAlreadyExists") {
          // Handle existing destination by copying contents individually
          const srcItems = await client.api(
            `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/root:/${templateName}/${packageName}:/children`
          ).get();

          const dstPath = `${botName}.gbai/${destinationName}`;
          const dst = await client.api(
            `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${libraryId}/drive/root:/${dstPath}`
          ).get();

          await CollectionUtil.asyncForEach(srcItems.value, async (item) => {
            await client.api(
              `https://graph.microsoft.com/v1.0/sites/${siteId}/lists/${templateLibraryId}/drive/items/${item.id}/copy`
            ).post({
              parentReference: {
                driveId: dst.parentReference.driveId,
                id: dst.id
              }
            });
          });
        } else {
          GBLog.error(`Failed to copy templates: ${error.message}`);
          throw error;
        }
      }
    }
    else if (storageMode === 'gbcluster') {

      // MinIO Implementation
      const minioClient = new Client({
        endPoint: process.env.DRIVE_SERVER,
        port: parseInt(process.env.DRIVE_PORT),
        useSSL: process.env.DRIVE_USE_SSL === 'true',
        accessKey: process.env.DRIVE_ACCESSKEY,
        secretKey: process.env.DRIVE_SECRET,
      });


      const bucketName = `${process.env.DRIVE_ORG_PREFIX}${botName}.gbai`.toLowerCase();
      const packageName = `${templateName.split('.')[0]}.${kind}`;
      const localTemplatePath = path.join(process.env.PWD, 'templates', templateName, packageName);
      const minioDestinationPath = `${botName}.${kind}`;

      const uploadDirectory = async (localPath: string, minioPath: string = '') => {

        // Ensure the bucket exists in local file system
        if (existsSync(localPath)) {

          const entries = await fs.readdir(localPath, { withFileTypes: true });

          for (const entry of entries) {
            const fullLocalPath = path.join(localPath, entry.name);
            const objectName = path.posix.join(minioPath, entry.name);

            if (entry.isDirectory()) {
              await uploadDirectory(fullLocalPath, objectName);
            } else {
              const fileContent = await fs.readFile(fullLocalPath);
              await minioClient.putObject(bucketName, objectName, fileContent);
              GBLog.info(`Uploaded ${objectName} to MinIO bucket ${bucketName}`);
            }
          }
        }
        else {
          GBLog.verbose(`Package ${localPath} does not exist on templates.`);
        }
      };

      await uploadDirectory(localTemplatePath, minioDestinationPath);
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

  public async shareWithEmail(bucketName, folder,  expiresInHours = 24 * 365) {
    const minioClient = new Client({
      endPoint: process.env.DRIVE_SERVER || 'localhost',
      port: parseInt(process.env.DRIVE_PORT || '9000', 10),
      useSSL: process.env.DRIVE_USE_SSL === 'true',
      accessKey: process.env.DRIVE_ACCESSKEY,
      secretKey: process.env.DRIVE_SECRET,
    });
  
      // Generate a time-limited access link (default: 24 hours)
      const presignedUrl = await minioClient.presignedGetObject(
        bucketName,
        folder,
        expiresInHours * 60 * 60 // Convert hours to seconds
      );
      return presignedUrl;
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

  public async createRootFolder(token: string, name: string,
    siteId: string, libraryId: string) {
    const storageMode = process.env.GB_MODE;

    if (storageMode === 'gbcluster') {
      // Minio implementation
      const minioClient = new Client({
        endPoint: process.env.DRIVE_SERVER,
        port: parseInt(process.env.DRIVE_PORT),
        useSSL: process.env.DRIVE_USE_SSL === 'true',
        accessKey: process.env.DRIVE_ACCESSKEY,
        secretKey: process.env.DRIVE_SECRET,
      });


      // Ensure bucket exists

      name = `${process.env.DRIVE_ORG_PREFIX}${name}`.toLowerCase();
      const bucketExists = await minioClient.bucketExists(name);
      if (!bucketExists) {
        await minioClient.makeBucket(name);
      }
      return { name: name, folder: {} }; // Return similar structure to MS Graph

    } else {
      // Original MS Graph implementation
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
  }


}
