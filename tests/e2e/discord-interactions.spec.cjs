const { test,expect } = require('@playwright/test');
const password='e2e-ci-only-secret!';

test('Discord linking is staff-only and unsigned callbacks cannot mutate tickets',async ({request})=>{
    const memberAuth=await request.post('/api/login',{data:{username:'fixture-user',password}});
    const staffAuth=await request.post('/api/login',{data:{username:'fixture-coach',password}});
    expect(memberAuth.status()).toBe(200);
    expect(staffAuth.status()).toBe(200);
    const member={Authorization:'Bearer '+(await memberAuth.json()).token};
    const staff={Authorization:'Bearer '+(await staffAuth.json()).token};
    expect((await request.get('/api/account/discord',{headers:member})).status()).toBe(403);
    expect((await request.post('/api/account/discord/code',{headers:member})).status()).toBe(403);
    expect((await request.delete('/api/account/discord',{headers:member})).status()).toBe(403);
    const result=await request.get('/api/account/discord',{headers:staff});
    expect(result.status()).toBe(200);
    const status=await result.json();
    expect(status.discord_user_id).toBeNull();
    expect(typeof status.enabled).toBe('boolean');
    if(!status.enabled){
        expect((await request.post('/api/account/discord/code',{headers:staff})).status()).toBe(503);
    }else{
        expect((await request.post('/api/account/discord/code',{headers:staff})).status()).toBe(200);
    }
    const forged=await request.post('/api/discord/interactions',{
        data:{type:2,guild_id:'999999999999999999',data:{name:'seppyl'},
            member:{user:{id:'123456789012345678'}}}
    });
    expect([401,503]).toContain(forged.status());
});

test('staff can open Discord account settings without exposing secrets',async ({page})=>{
    await page.goto('/login');
    await page.locator('input[autocomplete="username"]').fill('fixture-coach');
    await page.locator('input[type="password"]').fill(password);
    await page.locator('button[type="submit"]').click();
    await expect.poll(()=>new URL(page.url()).pathname).toBe('/trainings');
    await page.goto('/settings');
    await page.getByRole('tab',{name:'Discord'}).click();
    await expect(page.getByRole('heading',{name:'Discord'})).toBeVisible();
    await expect(page.locator('.discord-link-code')).toHaveCount(0);
});
