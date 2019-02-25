/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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

const Path = require('path');
const Fs = require('fs');
const _ = require('lodash');
const Parse = require('csv-parse');
const Async = require('async');
const UrlJoin = require('url-join');
const logger = require('../../../src/logger');

import { GBService, GBServiceCallback, IGBInstance } from 'botlib';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from '../models';

export class SecService extends GBService {

  public async importSecurityFile(localPath: string, instance: IGBInstance) {
    const security = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, 'security.json'), 'utf8')
    );
    security.groups.forEach(group => {
      const groupDb = GuaribasGroup.build({
        instanceId: instance.instanceId,
        displayName: group.displayName
      });
      groupDb.save().then(groupDb => {
        group.users.forEach(user => {
          const userDb = GuaribasUser.build({
            instanceId: instance.instanceId,
            groupId: groupDb.groupId,
            userName: user.userName
          });
          userDb.save().then(userDb => {
            const userGroup = GuaribasUserGroup.build();
            userGroup.groupId = groupDb.groupId;
            userGroup.userId = userDb.userId;
            userGroup.save();
          });
        });
      });
    });
  }

  public async ensureUser(
    instanceId: number,
    userSystemId: string,
    userName: string,
    address: string,
    channelName: string,
    displayName: string
  ): Promise<GuaribasUser> {
    return new Promise<GuaribasUser>(
      (resolve, reject) => {

        GuaribasUser.findOne({
          attributes: ['instanceId', 'internalAddress'],
          where: {
            instanceId: instanceId,
            userSystemId: userSystemId
          }
        }).then(user => {
          if (!user) {
            user = GuaribasUser.build();
          }
          user.userSystemId = userSystemId;
          user.userName = userName;
          user.displayName = displayName;
          user.internalAddress = address;
          user.email = userName;
          user.defaultChannel = channelName;
          user.save();
          resolve(user);
        }).error(reject);
      });
  }
}
