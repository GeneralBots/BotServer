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

import { GBLog, GBMinInstance, GBService } from 'botlib';
import { GBServer } from '../../../src/app';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBVMService } from '../../basic.gblib/services/GBVMService';
import { GuaribasSchedule } from '../../core.gbapp/models/GBModel';
import { FindOptions } from 'sequelize/types';

const cron = require('node-cron');

/**
 * @fileoverview Schedule Services.
 */

/**
 * Basic services for BASIC manipulation.
 */
export class ScheduleServices extends GBService {

  public async deleteScheduleIfAny(min: GBMinInstance, name: string) {

    const task = min["scheduleMap"] ? min["scheduleMap"][name] : null;

    if (task) {
      task.destroy();
      delete min["scheduleMap"][name];
    }

    const count = await GuaribasSchedule.destroy({
      where: {
        instanceId: min.instance.instanceId,
        name: name
      }
    });

    if (count > 0) {
      GBLog.info(`BASIC: Removed ${name} SET SCHEDULE and ${count} rows from storage on: ${min.botId}...`);
    }
  }

  /**
   * Finds and update user agent information to a next available person.
   */
  public async createOrUpdateSchedule(
    min: GBMinInstance,
    schedule: string,
    name: string
  ): Promise<GuaribasSchedule> {
    let record = await GuaribasSchedule.findOne({
      where: {
        instanceId: min.instance.instanceId,
        name: name
      }
    });

    if (record === null) {
      record = await GuaribasSchedule.create(<GuaribasSchedule>{
        instanceId: min.instance.instanceId,
        name: name,
        schedule: schedule
      });
    } else {
      record.schedule = schedule;
      await record.save();
    }

    this.ScheduleItem(record, min);

    return record;
  }


  /**
 * Load all cached schedule from BASIC SET SCHEDULE keyword.
 */
  public async scheduleAll() {
   
    let schedules;
    try {
      schedules = await GuaribasSchedule.findAll();
      await CollectionUtil.asyncForEach(schedules, async item => {
        let min: GBMinInstance = GBServer.globals.minInstances.filter(
          p => p.instance.instanceId === item.instanceId
        )[0];

        if (min){
          this.ScheduleItem(item, min);
        }
      });
    } catch (error) {
      throw new Error(`Cannot schedule: ${error.message}.`);
    }
    return schedules;
  }

  private ScheduleItem(item: GuaribasSchedule, min: GBMinInstance) {
    GBLog.info(`Scheduling ${item.name} on ${min.botId}...`);
    try {
      const options = {
        scheduled: true,
        timezone: 'America/Sao_Paulo'
      };

      const task = min["scheduleMap"][item.name];
      if (task) {
        task.stop();
        min["scheduleMap"][item.name] = null;
      }

      min["scheduleMap"][item.name] = cron.schedule(
        item.schedule,
        function () {
          const finalData = async () => {
            let script = item.name;
            let min: GBMinInstance = GBServer.globals.minInstances.filter(
              p => p.instance.instanceId === item.instanceId
            )[0];
            await GBVMService.callVM(script, min, null, null, false);
          };
          (async () => {
            await finalData();
          })();
        }, options
      );
      GBLog.info(`Running .gbdialog word ${item.name} on:${item.schedule}...`);
    } catch (error) { }
  }
}
