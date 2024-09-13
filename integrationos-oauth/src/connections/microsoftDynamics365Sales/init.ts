import axios from 'axios';
import qs from 'qs';
import { DataObject, OAuthResponse } from '../../lib/types';

export const init = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const requestBody = {
            grant_type: 'client_credentials',
            client_id: body.clientId,
            client_secret: body.clientSecret,
            resource:
                body.metadata?.formData
                    ?.MICROSOFT_DYNAMICS_365_SALES_ORGANIZATION_URI,
        };

        const response = await axios({
            url: `https://login.windows.net/${body.metadata?.formData?.MICROSOFT_DYNAMICS_365_SALES_TENANT_ID}/oauth2/token`,
            method: 'POST',
            headers: { 'content-type': 'application/x-www-form-urlencoded' },
            data: qs.stringify(requestBody),
        });

        const {
            data: { access_token, expires_in, token_type, resource },
        } = response;

        return {
            accessToken: access_token,
            refreshToken: '',
            expiresIn: +expires_in,
            tokenType: token_type,
            meta: {
                resource,
            },
        };
    } catch (error) {
        throw new Error(
            `Error fetching access token for Microsoft Dynamics 365 Sales: ${error}`,
        );
    }
};
