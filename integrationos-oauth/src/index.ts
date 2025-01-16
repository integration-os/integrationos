import 'dotenv/config';
import path from 'path';
import express, { Express, Response } from 'express';

import { checkExistence, getProjectPath, toCamelCase } from './lib/helpers';

const app: Express = express();

app.use(express.json());
app.use(express.urlencoded({ extended: true }));

const port = Number(process.env.EXPRESS_SERVER_PORT || 3000);
const hostname = process.env.EXPRESS_SERVER_HOSTNAME || '0.0.0.0';

const SERVER_OAUTH_PATH = './connections';

app.get('/', (_, res: Response) => {
    res.send('Pica');
});

app.listen(port, hostname, () => {
    console.log(
        `Pica Oauth Server is running on http://${hostname}:${port}`,
    );
});

app.route('/oauth/:platform/init')
    .get(async (req, res) => {
        res.send(
            `To Init OAuth for Platform: <b>${await toCamelCase(
                req.params.platform,
            )}</b>, perform a <b>POST</b> request!`,
        );
    })
    .post(async (req, res) => {
        try {
            const { platform } = req.params;
            const platformOAuthPath = path.join(
                await getProjectPath(),
                ...SERVER_OAUTH_PATH.split('/'),
                await toCamelCase(platform),
            );

            if (!(await checkExistence(platformOAuthPath))) {
                return res.status(404).send({
                    message: `Error: OAuth does not exist for ${platform}!`,
                });
            }

            const { init } = require(`${platformOAuthPath}/init`);

            if (typeof init !== 'function') {
                return res.status(500).send({
                    message: `Error: Missing init function for ${platform}!`,
                });
            }

            const response = await init({
                headers: req.headers,
                params: req.params,
                body: req.body,
            });

            res.send(response);
        } catch (error) {
            console.error('Error during OAuth initialization:', error);
            res.status(500).send({ message: 'Internal Server Error' });
        }
    });

app.route('/oauth/:platform/refresh')
    .get(async (req, res) => {
        res.send(
            `To Refresh OAuth for Platform: <b>${await toCamelCase(
                req.params.platform,
            )}</b>, perform a <b>POST</b> request!`,
        );
    })
    .post(async (req, res) => {
        try {
            const { platform } = req.params;

            const platformOAuthPath = path.join(
                await getProjectPath(),
                ...SERVER_OAUTH_PATH.split('/'),
                await toCamelCase(platform),
            );

            if (!(await checkExistence(platformOAuthPath))) {
                return res.status(404).send({
                    message: `Error: OAuth does not exist for ${platform}!`,
                });
            }

            const { refresh } = require(`${platformOAuthPath}/refresh`);

            if (typeof refresh !== 'function') {
                return res.status(500).send({
                    message: `Error: Missing refresh function for ${platform}!`,
                });
            }

            const response = await refresh({
                headers: req.headers,
                params: req.params,
                body: req.body,
            });

            res.send(response);
        } catch (error) {
            console.error('Error during OAuth refresh:', error);
            res.status(500).send({ message: 'Internal Server Error' });
        }
    });
