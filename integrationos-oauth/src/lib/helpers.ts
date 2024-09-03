import { promises } from 'node:fs';
import path from 'path';

export const checkExistence = async (path: string) => {
    try {
        await promises.access(path);

        return true;
    } catch (error) {
        return false;
    }
};

export const getProjectPath = async () => path.join(__dirname, '..');

export const toCamelCase = async (input: string) => {
    const words = input.split(/[^a-zA-Z0-9]+/).filter((word) => word.length);

    for (let i = 1; i < words.length; i++) {
        words[i] =
            words[i][0].toUpperCase() + words[i].substring(1).toLowerCase();
    }

    const result = words.join('');

    return result[0].toLowerCase() + result.substring(1);
};

export const convertToTimestamp = async (dateString: string): Promise<number> =>
    new Date(dateString).getTime();

export const base64encode = (val: string) => btoa(val);

export const differenceInSeconds = (argDate: Date) => {
    const currentDate = new Date().getTime();
    const dateArg = new Date(argDate).getTime();

    // Calculate the difference in milliseconds
    const differenceInMillis = dateArg - currentDate;

    // Convert milliseconds to seconds
    const differenceInSeconds = differenceInMillis / 1000;

    return Math.floor(differenceInSeconds);
};

export const generateBasicHeaders = (
    clientId: string,
    clientSecret: string,
) => {
    const credentials = clientId + ':' + clientSecret;
    const encodedCredentials = Buffer.from(credentials).toString('base64');

    return {
        Authorization: 'Basic ' + encodedCredentials,
        'Content-Type': 'application/x-www-form-urlencoded',
    };
};
