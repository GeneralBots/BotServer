const Fs = require('fs');
const UrlJoin = require('url-join');

import { GBService, IGBInstance } from 'botlib';
import { GuaribasGroup, GuaribasUser, GuaribasUserGroup } from '../models';

export class SecService extends GBService {
  public async importSecurityFile(localPath: string, instance: IGBInstance) {
    const security = JSON.parse(Fs.readFileSync(UrlJoin(localPath, 'security.json'), 'utf8'));
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
    return new Promise<GuaribasUser>((resolve, reject) => {
      GuaribasUser.findOne({
        attributes: ['instanceId', 'internalAddress'],
        where: {
          instanceId: instanceId,
          userSystemId: userSystemId
        }
      })
        .then(user => {
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
        })
        .error(reject);
    });
  }
}
