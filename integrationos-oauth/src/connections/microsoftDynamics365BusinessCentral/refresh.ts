import axios from 'axios';
import qs from 'qs';
import { DataObject, OAuthResponse } from '../../lib/types';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_CLIENT_ID: client_id,
            OAUTH_CLIENT_SECRET: client_secret,
            OAUTH_REQUEST_PAYLOAD: { formData },
            OAUTH_METADATA,
        } = body;
        const requestBody = {
            grant_type: 'client_credentials',
            client_id,
            client_secret,
            scope: 'https://api.businesscentral.dynamics.com/.default',
        };

        const response = await axios({
            url: `https://login.microsoftonline.com/${formData?.BUSINESS_CENTRAL_TENANT_ID}/oauth2/v2.0/token`,
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            data: qs.stringify(requestBody),
        });

        const {
            data: { access_token, expires_in, token_type },
        } = response;

        return {
            accessToken: access_token,
            refreshToken: '',
            expiresIn: +expires_in,
            tokenType: token_type,
            meta: {
                ...OAUTH_METADATA?.meta,
            },
        };
    } catch (error) {
        throw new Error(
            `Error fetching refresh token for Microsoft Dynamics 365 Business Central: ${error}`,
        );
    }
};
