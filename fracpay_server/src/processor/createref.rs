/****************************************************************
 * Fracpay server CreateREF instruction process    	   
 * blairmunroakusa@.0322.anch.AK			     
 ****************************************************************/

#![allow(non_snake_case)]
use solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
        msg,
    };
use bit_vec::BitVec;
use crate::{
        error::FracpayError,
        processor::{
            run::Processor,
            utility::*,
        },
        state::{
            constants::*,
            MAIN::*,
            PIECE::*,
            REF::*,
        },
    };

impl Processor {

    pub fn process_create_ref<'a>(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'a>],
        bumpREF: u8,
        seedREF: Vec<u8>,
        REFslug: Vec<u8>,
    ) -> ProgramResult {

        // get accounts
        let (operator, rent, pda) = get_accounts(accounts)?;

        // check to make sure tx operator is signer
        if !operator.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // get MAIN info
        let MAINinfo = MAIN::unpack_unchecked(&pda.MAIN.try_borrow_data()?)?;

        // check to make sure tx operator is authorized MAIN operator
        if MAINinfo.operator != *operator.key {
            msg!("operator doesn't control MAIN.");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // get PIECE info
        let mut PIECEinfo = PIECE::unpack_unchecked(&pda.PIECE.try_borrow_data()?)?;
        
        // check to make sure tx operator is authorized PIECE operator
        if PIECEinfo.operator != *operator.key {
            msg!("Operator doesn't control PIECE.");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // make sure seed is correct count number
        if PIECEinfo.refcount != (check_seed(&seedREF) - 1) {
            msg!{"This REF pda is out of order."}
            return Err(FracpayError::AccountCreationAttemptError.into());
        }

        // calculate rent
        let rentREF = rent.minimum_balance(SIZE_REF.into());

        // create pdaREF
        invoke_signed(
        &system_instruction::create_account(
            &operator.key,
            &pda.REF.key,
            rentREF,
            SIZE_REF.into(),
            program_id
        ),
        &[
            operator.clone(),
            pda.REF.clone()
        ],
        &[&[&seedREF, &[bumpREF]]]
        )?;
        msg!("Successfully created pdaREF");

        // update REF count
        PIECEinfo.refcount = PIECEinfo.refcount + 1;
        PIECE::pack(PIECEinfo, &mut pda.PIECE.try_borrow_mut_data()?)?;

        // get REF info
        let mut REFinfo = REF::unpack_unchecked(&pda.REF.try_borrow_data()?)?;

        // set flags
        let mut FLAGS = BitVec::from_elem(16, false);
        FLAGS.set(0, false); // REF account is 0100
        FLAGS.set(1, true);  
        FLAGS.set(2, false);
        FLAGS.set(3, false); 
        FLAGS.set(4, false); // not connected
        FLAGS.set(5, false); // not initialized
        FLAGS.set(6, false); // not reflected

        // initialize REF account data
        REFinfo.flags = pack_flags(FLAGS);
        REFinfo.target = *operator.key;
        REFinfo.fract = 0;  // new ref get's 0% by default
        REFinfo.netsum = 0;
        REFinfo.refslug = pack_refslug(REFslug);
        REF::pack(REFinfo, &mut pda.REF.try_borrow_mut_data()?)?;

        Ok(())
    }
}
