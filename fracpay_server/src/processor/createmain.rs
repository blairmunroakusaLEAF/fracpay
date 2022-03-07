/****************************************************************
 * Fracpay server CreateMAIN instruction process    			*
 * blairmunroakusa@.0322.anch.AK			                	*
 ****************************************************************/

#![allow(non_snake_case)]
use solana_program::{
        msg,
        system_instruction,
        account_info::{
            next_account_info,
            AccountInfo
        },
        entrypoint::ProgramResult,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        sysvar::{
            Sysvar,
            rent::Rent,
        },
    };
use bit_vec::BitVec;
use std::array::TryFromSliceError;
use crate::{
        state::{
            constants::*,
            MAIN::*,
            PIECE::*,
            REF::*,
        },
    };
use crate::processor::{
        run::Processor,
        utility::*,
    };

impl Processor {

    pub fn process_create_main(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        bumpMAIN: u8,
        seedMAIN: Vec<u8>,
        bumpPIECE: u8,
        seedPIECE: Vec<u8>,
        bumpREF: u8,
        seedREF: Vec<u8>,
    ) -> ProgramResult {

        let account_info_iter = &mut accounts.iter();
        
        // account #1
        let operator = next_account_info(account_info_iter)?;

        if !operator.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        // account #2
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        // account #3
        let pdaMAIN = next_account_info(account_info_iter)?;
        
        // account #4
        let pdaPIECE = next_account_info(account_info_iter)?;

        // account #5
        let pdaREF = next_account_info(account_info_iter)?;

        // prep to create MAIN pda
        let rentMAIN = rent.minimum_balance(SIZE_MAIN.into());
        
        // prep to create self PIECE pda
        let rentPIECE = rent.minimum_balance(SIZE_PIECE.into());

        // prep to create self REF pda
        let rentREF = rent.minimum_balance(SIZE_REF.into());

       
        // create pdaMAIN
        invoke_signed(
        &system_instruction::create_account(
            &operator.key,
            &pdaMAIN.key,
            rentMAIN,
            SIZE_MAIN.into(),
            &program_id
        ),
        &[
            operator.clone(),
            pdaMAIN.clone()
        ],
        &[&[&seedMAIN, &[bumpMAIN]]]
        )?;
        msg!("Successfully created pdaMAIN");

        // create pdaPIECEself
        invoke_signed(
        &system_instruction::create_account(
            &operator.key,
            &pdaPIECE.key,
            rentPIECE,
            SIZE_PIECE.into(),
            &program_id
        ),
        &[
            operator.clone(),
            pdaPIECE.clone()
        ],
        &[&[&seedPIECE, &[bumpPIECE]]]
        )?;
        msg!("Successfully created pdaPIECE");


        // create pdaREFself
        invoke_signed(
        &system_instruction::create_account(
            &operator.key,
            &pdaREF.key,
            rentREF,
            SIZE_REF.into(),
            program_id
        ),
        &[
            operator.clone(),
            pdaREF.clone()
        ],
        &[&[&seedREF, &[bumpREF]]]
        )?;
        msg!("Successfully created pdaREF");


        // initialize MAIN account data

        let mut MAINinfo = MAIN::unpack_unchecked(&pdaMAIN.try_borrow_data()?)?;

        let mut FLAGS = BitVec::from_elem(16, false);
        FLAGS.set(0, false); // MAIN account is 0000
        FLAGS.set(1, false);
        FLAGS.set(2, false); 
        FLAGS.set(3, false); 

        MAINinfo.flags = pack_flags(FLAGS);
        MAINinfo.operator = *operator.key;
        MAINinfo.balance = 0;
        MAINinfo.netsum = 0;
        MAINinfo.piececount = 0;

        MAIN::pack(MAINinfo, &mut pdaMAIN.try_borrow_mut_data()?)?;


        // initialize self PIECE account data
        
        let mut PIECEinfo = PIECE::unpack_unchecked(&pdaPIECE.try_borrow_data()?)?;

        let mut FLAGS = BitVec::from_elem(16, false);
        FLAGS.set(0, false); // PIECE self account is 0001
        FLAGS.set(1, false);
        FLAGS.set(2, false); 
        FLAGS.set(3, true); 

        PIECEinfo.flags = pack_flags(FLAGS);
        PIECEinfo.operator = *operator.key;
        PIECEinfo.balance = 0;
        PIECEinfo.netsum = 0;
        PIECEinfo.refcount = 0;
        { 
            type VecInput = Vec<u8>;
            type PieceslugOutput = [u8; PIECESLUG_LEN];
            fn package_slug(vector: VecInput) -> Result<PieceslugOutput, TryFromSliceError> {
                vector.as_slice().try_into()
            }
            let mut PIECEslug_bytes: Vec<u8>;
            PIECEslug_bytes = seedMAIN.to_vec();
            let mut zeros: Vec<u8> = vec![0; PIECESLUG_LEN - PIECEslug_bytes.len()];
            PIECEslug_bytes.append(&mut zeros);
            PIECEinfo.pieceslug = package_slug(PIECEslug_bytes).unwrap();
        }

        PIECE::pack(PIECEinfo, &mut pdaPIECE.try_borrow_mut_data()?)?;


        // initialize self REF account data

        let mut REFinfo = REF::unpack_unchecked(&pdaREF.try_borrow_data()?)?;

        let mut FLAGS = BitVec::from_elem(16, false);
        FLAGS.set(0, false); // REF self account is 0010
        FLAGS.set(1, false);
        FLAGS.set(2, true);
        FLAGS.set(4, false);

        REFinfo.flags = pack_flags(FLAGS);
        REFinfo.target = *operator.key;
        REFinfo.fract = 100_000_000;    // new self-ref gets 100% by default
        REFinfo.netsum = 0;
        {
            let slug = "SELF_REFERENCE";
            type VecInput = Vec<u8>;
            type RefslugOutput = [u8; REFSLUG_LEN];
            let mut REFslug_bytes: Vec<u8>;
            fn package_slug(vector: VecInput) -> Result<RefslugOutput, TryFromSliceError> {
                vector.as_slice().try_into()
            }
            REFslug_bytes = slug.as_bytes().to_vec();
            let mut zeros: Vec<u8> = vec![0; REFSLUG_LEN - REFslug_bytes.len()];
            REFslug_bytes.append(&mut zeros);
            REFinfo.refslug = package_slug(REFslug_bytes).unwrap();
        }

        REF::pack(REFinfo, &mut pdaREF.try_borrow_mut_data()?)?;


        Ok(())
    }
}

