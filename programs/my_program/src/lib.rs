use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program::invoke,
    system_instruction,
    rent::Rent,
    sysvar::Sysvar,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let liquidity_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let treasury = next_account_info(accounts_iter)?;

    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        0 => add_liquidity(program_id, payer, liquidity_account, system_program, instruction_data),
        1 => withdraw_liquidity(payer, liquidity_account, treasury),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn add_liquidity<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    liquidity_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    instruction_data: &[u8],
) -> ProgramResult {
    let fee_lamports = 2_000_000; // 0.002 SOL
    let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());

    if !liquidity_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent_exempt = Rent::get()?.minimum_balance(8);
    invoke(
        &system_instruction::create_account(
            payer.key,
            liquidity_account.key,
            rent_exempt,
            8,
            program_id,
        ),
        &[payer.clone(), liquidity_account.clone(), system_program.clone()],
    )?;

    invoke(
        &system_instruction::transfer(payer.key, liquidity_account.key, fee_lamports + amount),
        &[payer.clone(), liquidity_account.clone(), system_program.clone()],
    )?;

    liquidity_account.try_borrow_mut_data()?[0..8].copy_from_slice(&amount.to_le_bytes());
    msg!("Added {} lamports", amount);
    Ok(())
}

fn withdraw_liquidity<'a>(
    payer: &AccountInfo<'a>,
    liquidity_account: &AccountInfo<'a>,
    treasury: &AccountInfo<'a>,
) -> ProgramResult {
    let amount = u64::from_le_bytes(liquidity_account.data.borrow()[0..8].try_into().unwrap());
    let total_lamports = **liquidity_account.lamports.borrow();
    let fee_lamports = 2_000_000;
    let user_amount = total_lamports - fee_lamports;

    **liquidity_account.lamports.borrow_mut() = 0;
    **payer.lamports.borrow_mut() += user_amount;
    **treasury.lamports.borrow_mut() += fee_lamports;

    msg!("Withdrew {} lamports", amount);
    Ok(())
}
